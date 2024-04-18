use alloc::{string::ToString, vec::Vec};

use miden_objects::{
    accounts::AccountId,
    assembly::{Assembler, AssemblyContext, ProgramAst},
    transaction::{OutputNote, OutputNotes, TransactionOutputs},
    utils::{group_slice_elements, serde::DeserializationError},
    vm::{AdviceMap, ProgramInfo, StackInputs, StackOutputs},
    Digest, Felt, TransactionOutputError, Word,
};
use miden_stdlib::StdLibrary;

use super::MidenLib;

pub mod memory;

mod events;
pub use events::{TransactionEvent, TransactionTrace};

mod inputs;
pub use inputs::ToTransactionKernelInputs;

mod outputs;
pub use outputs::{
    parse_final_account_stub, FINAL_ACCOUNT_HASH_WORD_IDX, OUTPUT_NOTES_COMMITMENT_WORD_IDX,
};

mod errors;
pub use errors::{
    TransactionEventParsingError, TransactionKernelError, TransactionTraceParsingError,
};

// TRANSACTION KERNEL
// ================================================================================================

pub struct TransactionKernel;

impl TransactionKernel {
    // KERNEL SOURCE CODE
    // --------------------------------------------------------------------------------------------

    /// Returns MASM source code which encodes the transaction kernel system procedures.
    pub fn kernel() -> &'static str {
        include_str!("../../asm/kernels/transaction/api.masm")
    }

    /// Returns an AST of the transaction kernel executable program.
    ///
    /// # Errors
    /// Returns an error if deserialization of the binary fails.
    pub fn main() -> Result<ProgramAst, DeserializationError> {
        let kernel_bytes =
            include_bytes!(concat!(env!("OUT_DIR"), "/assets/kernels/transaction.masb"));
        ProgramAst::from_bytes(kernel_bytes)
    }

    /// Returns [ProgramInfo] for the transaction kernel executable program.
    ///
    /// # Panics
    /// Panics if the transaction kernel source is not well-formed.
    pub fn program_info() -> ProgramInfo {
        // TODO: construct kernel_main and kernel using lazy static or at build time
        let assembler = Self::assembler();
        let main_ast = TransactionKernel::main().expect("main is well formed");
        let kernel_main = assembler
            .compile_in_context(&main_ast, &mut AssemblyContext::for_program(Some(&main_ast)))
            .expect("main is well formed");

        ProgramInfo::new(kernel_main.hash(), assembler.kernel().clone())
    }

    // ASSEMBLER CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new Miden assembler instantiated with the transaction kernel and loaded with the
    /// Miden stdlib as well as with midenlib.
    pub fn assembler() -> Assembler {
        Assembler::default()
            .with_library(&MidenLib::default())
            .expect("failed to load miden-lib")
            .with_library(&StdLibrary::default())
            .expect("failed to load std-lib")
            .with_kernel(Self::kernel())
            .expect("kernel must be well formed")
    }

    // STACK INPUTS / OUTPUTS
    // --------------------------------------------------------------------------------------------

    /// Returns the input stack required to execute the transaction kernel.
    ///
    /// This includes the input notes commitment, the account hash, the account id, and the block
    /// hash.
    ///
    /// Stack: [BH, acct_id, IAH, NC]
    ///
    /// Where:
    /// - BH is the latest known block hash at the time of transaction execution.
    /// - acct_id is the account id of the account that the transaction is being executed against.
    /// - IAH is the hash of account state immediately before the transaction is executed. For
    ///   newly created accounts, initial state hash is provided as [ZERO; 4].
    /// - NC is a commitment to the input notes. This is a sequential hash of all (nullifier, ZERO)
    ///   tuples for the notes consumed by the transaction.
    pub fn build_input_stack(
        acct_id: AccountId,
        init_acct_hash: Digest,
        input_notes_hash: Digest,
        block_hash: Digest,
    ) -> StackInputs {
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
    /// - FAH is the final account hash of the account that the transaction is being
    ///   executed against.
    pub fn parse_output_stack(stack: &StackOutputs) -> (Digest, Digest) {
        let output_notes_hash = stack
            .get_stack_word(OUTPUT_NOTES_COMMITMENT_WORD_IDX * 4)
            .expect("first word missing")
            .into();
        let final_account_hash = stack
            .get_stack_word(FINAL_ACCOUNT_HASH_WORD_IDX * 4)
            .expect("second word missing")
            .into();

        (final_account_hash, output_notes_hash)
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
    /// - FAH is the final account hash of the account that the transaction is being
    ///   executed against.
    ///
    /// The actual data describing the new account state and output notes is expected to be located
    /// in the provided advice map under keys CNC and FAH.
    pub fn from_transaction_parts(
        stack: &StackOutputs,
        adv_map: &AdviceMap,
        output_notes: Vec<OutputNote>,
    ) -> Result<TransactionOutputs, TransactionOutputError> {
        let (final_acct_hash, output_notes_hash) = Self::parse_output_stack(stack);

        // parse final account state
        let final_account_data: &[Word] = group_slice_elements(
            adv_map
                .get(&final_acct_hash)
                .ok_or(TransactionOutputError::FinalAccountDataNotFound)?,
        );
        let account = parse_final_account_stub(final_account_data)
            .map_err(TransactionOutputError::FinalAccountStubDataInvalid)?;

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
