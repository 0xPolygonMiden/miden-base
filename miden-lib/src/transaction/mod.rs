use alloc::{string::ToString, sync::Arc, vec::Vec};

#[cfg(feature = "testing")]
use miden_objects::accounts::AccountCode;
use miden_objects::{
    accounts::AccountId,
    assembly::{Assembler, DefaultSourceManager, KernelLibrary},
    transaction::{
        OutputNote, OutputNotes, TransactionArgs, TransactionInputs, TransactionOutputs,
    },
    utils::{group_slice_elements, serde::Deserializable},
    vm::{AdviceInputs, AdviceMap, Program, ProgramInfo, StackInputs, StackOutputs},
    Digest, Felt, TransactionOutputError, Word, EMPTY_WORD,
};
use miden_stdlib::StdLibrary;

use super::MidenLib;

pub mod memory;

mod events;
pub use events::{TransactionEvent, TransactionTrace};

mod inputs;

mod outputs;
pub use outputs::{
    parse_final_account_header, FINAL_ACCOUNT_HASH_WORD_IDX, OUTPUT_NOTES_COMMITMENT_WORD_IDX,
};

mod errors;
pub use errors::{
    TransactionEventParsingError, TransactionKernelError, TransactionTraceParsingError,
};

mod procedures;

// Include procedure hashes generated in build.rs
#[rustfmt::skip]
mod procedures_v0;

// CONSTANTS
// ================================================================================================

const KERNEL_LIB_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/kernels/tx_kernel.masl"));
const KERNEL_MAIN_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/assets/kernels/tx_kernel.masb"));

// TRANSACTION KERNEL
// ================================================================================================

pub struct TransactionKernel;

impl TransactionKernel {
    // KERNEL SOURCE CODE
    // --------------------------------------------------------------------------------------------

    /// Returns a library with the transaction kernel system procedures.
    ///
    /// # Panics
    /// Panics if the transaction kernel source is not well-formed.
    pub fn kernel() -> KernelLibrary {
        // TODO: make this static
        KernelLibrary::read_from_bytes(KERNEL_LIB_BYTES)
            .expect("failed to deserialize transaction kernel library")
    }

    /// Returns an AST of the transaction kernel executable program.
    ///
    /// # Panics
    /// Panics if the transaction kernel source is not well-formed.
    pub fn main() -> Program {
        // TODO: make static
        Program::read_from_bytes(KERNEL_MAIN_BYTES)
            .expect("failed to deserialize transaction kernel runtime")
    }

    /// Returns [ProgramInfo] for the transaction kernel executable program.
    ///
    /// # Panics
    /// Panics if the transaction kernel source is not well-formed.
    pub fn program_info() -> ProgramInfo {
        // TODO: make static
        let program_hash = Self::main().hash();
        let kernel = Self::kernel().kernel().clone();

        ProgramInfo::new(program_hash, kernel)
    }

    /// Transforms the provided [TransactionInputs] and [TransactionArgs] into stack and advice
    /// inputs needed to execute a transaction kernel for a specific transaction.
    ///
    /// If `init_advice_inputs` is provided, they will be included in the returned advice inputs.
    pub fn prepare_inputs(
        tx_inputs: &TransactionInputs,
        tx_args: &TransactionArgs,
        init_advice_inputs: Option<AdviceInputs>,
    ) -> (StackInputs, AdviceInputs) {
        let account = tx_inputs.account();

        let stack_inputs = TransactionKernel::build_input_stack(
            account.id(),
            account.init_hash(),
            tx_inputs.input_notes().commitment(),
            tx_inputs.block_header().hash(),
        );

        let mut advice_inputs = init_advice_inputs.unwrap_or_default();
        inputs::extend_advice_inputs(tx_inputs, tx_args, &mut advice_inputs);

        (stack_inputs, advice_inputs)
    }

    // ASSEMBLER CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new Miden assembler instantiated with the transaction kernel and loaded with the
    /// Miden stdlib as well as with miden-lib.
    pub fn assembler() -> Assembler {
        let source_manager = Arc::new(DefaultSourceManager::default());
        Assembler::with_kernel(source_manager, Self::kernel())
            .with_library(StdLibrary::default())
            .expect("failed to load std-lib")
            .with_library(MidenLib::default())
            .expect("failed to load miden-lib")
    }

    // STACK INPUTS / OUTPUTS
    // --------------------------------------------------------------------------------------------

    /// Returns the stack with the public inputs required by the transaction kernel.
    ///
    /// The initial stack is defined:
    ///
    /// ```text
    /// [
    ///     BLOCK_HASH,
    ///     acct_id,
    ///     INITIAL_ACCOUNT_HASH,
    ///     INPUT_NOTES_COMMITMENT,
    /// ]
    /// ```
    ///
    /// Where:
    /// - BLOCK_HASH, reference block for the transaction execution.
    /// - acct_id, the account that the transaction is being executed against.
    /// - INITIAL_ACCOUNT_HASH, account state prior to the transaction, EMPTY_WORD for new accounts.
    /// - INPUT_NOTES_COMMITMENT, see `transaction::api::get_input_notes_commitment`.
    pub fn build_input_stack(
        acct_id: AccountId,
        init_acct_hash: Digest,
        input_notes_hash: Digest,
        block_hash: Digest,
    ) -> StackInputs {
        // Note: Must be kept in sync with the transaction's kernel prepare_transaction procedure
        let mut inputs: Vec<Felt> = Vec::with_capacity(13);
        inputs.extend(input_notes_hash);
        inputs.extend_from_slice(init_acct_hash.as_elements());
        inputs.push(acct_id.into());
        inputs.extend_from_slice(block_hash.as_elements());
        StackInputs::new(inputs)
            .map_err(|e| e.to_string())
            .expect("Invalid stack input")
    }

    pub fn build_output_stack(final_acct_hash: Digest, output_notes_hash: Digest) -> StackOutputs {
        let mut outputs: Vec<Felt> = Vec::with_capacity(9);
        outputs.extend(final_acct_hash);
        outputs.extend(output_notes_hash);
        outputs.reverse();
        StackOutputs::new(outputs, Vec::new())
            .map_err(|e| e.to_string())
            .expect("Invalid stack output")
    }

    /// Extracts transaction output data from the provided stack outputs.
    ///
    /// The data on the stack is expected to be arranged as follows:
    ///
    /// Stack: [CNC, FAH]
    ///
    /// Where:
    /// - CNC is the commitment to the notes created by the transaction.
    /// - FAH is the final account hash of the account that the transaction is being executed
    ///   against.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Words 3 and 4 on the stack are not 0.
    /// - Overflow addresses are not empty.
    pub fn parse_output_stack(
        stack: &StackOutputs,
    ) -> Result<(Digest, Digest), TransactionOutputError> {
        let output_notes_hash = stack
            .get_stack_word(OUTPUT_NOTES_COMMITMENT_WORD_IDX * 4)
            .expect("first word missing")
            .into();
        let final_account_hash = stack
            .get_stack_word(FINAL_ACCOUNT_HASH_WORD_IDX * 4)
            .expect("second word missing")
            .into();

        // make sure that the stack has been properly cleaned
        if stack.get_stack_word(8).expect("third word missing") != EMPTY_WORD {
            return Err(TransactionOutputError::OutputStackInvalid(
                "Third word on output stack should consist only of ZEROs".into(),
            ));
        }
        if stack.get_stack_word(12).expect("fourth word missing") != EMPTY_WORD {
            return Err(TransactionOutputError::OutputStackInvalid(
                "Fourth word on output stack should consist only of ZEROs".into(),
            ));
        }
        if stack.has_overflow() {
            return Err(TransactionOutputError::OutputStackInvalid(
                "Output stack should not have overflow addresses".into(),
            ));
        }

        Ok((final_account_hash, output_notes_hash))
    }

    // TRANSACTION OUTPUT PARSER
    // --------------------------------------------------------------------------------------------

    /// Returns [TransactionOutputs] constructed from the provided output stack and advice map.
    ///
    /// The output stack is expected to be arrange as follows:
    ///
    /// Stack: [CNC, FAH]
    ///
    /// Where:
    /// - CNC is the commitment to the notes created by the transaction.
    /// - FAH is the final account hash of the account that the transaction is being executed
    ///   against.
    ///
    /// The actual data describing the new account state and output notes is expected to be located
    /// in the provided advice map under keys CNC and FAH.
    pub fn from_transaction_parts(
        stack: &StackOutputs,
        adv_map: &AdviceMap,
        output_notes: Vec<OutputNote>,
    ) -> Result<TransactionOutputs, TransactionOutputError> {
        let (final_acct_hash, output_notes_hash) = Self::parse_output_stack(stack)?;

        // parse final account state
        let final_account_data: &[Word] = group_slice_elements(
            adv_map
                .get(&final_acct_hash)
                .ok_or(TransactionOutputError::FinalAccountDataNotFound)?,
        );
        let account = parse_final_account_header(final_account_data)
            .map_err(TransactionOutputError::FinalAccountHeaderDataInvalid)?;

        // validate output notes
        let output_notes = OutputNotes::new(output_notes)?;
        if output_notes_hash != output_notes.commitment() {
            return Err(TransactionOutputError::OutputNotesCommitmentInconsistent(
                output_notes_hash,
                output_notes.commitment(),
            ));
        }

        Ok(TransactionOutputs { account, output_notes })
    }
}

#[cfg(feature = "testing")]
impl TransactionKernel {
    const KERNEL_TESTING_LIB_BYTES: &'static [u8] =
        include_bytes!(concat!(env!("OUT_DIR"), "/assets/kernels/kernel_library.masl"));

    pub fn kernel_as_library() -> miden_objects::assembly::Library {
        miden_objects::assembly::Library::read_from_bytes(Self::KERNEL_TESTING_LIB_BYTES)
            .expect("failed to deserialize transaction kernel library")
    }

    /// Contains code to get an instance of the [Assembler] that should be used in tests.
    ///
    /// This assembler is similar to the assembler used to assemble the kernel and transactions,
    /// with the difference that it also includes an extra library on the namespace of `kernel`.
    /// The `kernel` library is added separately because even though the library (`api.masm`) and
    /// the kernel binary (`main.masm`) include this code, it is not exposed explicitly. By adding
    /// it separately, we can expose procedures from `/lib` and test them individually.
    pub fn testing_assembler() -> Assembler {
        let source_manager = Arc::new(DefaultSourceManager::default());
        let kernel_library = Self::kernel_as_library();

        Assembler::with_kernel(source_manager, Self::kernel())
            .with_library(StdLibrary::default())
            .expect("failed to load std-lib")
            .with_library(MidenLib::default())
            .expect("failed to load miden-lib")
            .with_library(kernel_library)
            .expect("failed to load kernel library (/lib)")
    }

    /// Returns the testing assembler, and additionally contains the library for
    /// [AccountCode::mock_library()], which is a mock wallet used in tests.
    pub fn testing_assembler_with_mock_account() -> Assembler {
        let assembler = Self::testing_assembler();
        let library = AccountCode::mock_library(assembler.clone());

        assembler.with_library(library).expect("failed to add mock account code")
    }
}
