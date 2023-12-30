use assembly::{ast::ProgramAst, utils::DeserializationError, Assembler};
use miden_objects::{
    accounts::AccountId,
    notes::Nullifier,
    transaction::{InputNote, OutputNotes, TransactionOutputs},
    utils::{
        collections::{BTreeMap, Vec},
        group_slice_elements,
    },
    vm::{StackInputs, StackOutputs},
    Digest, Felt, Hasher, StarkField, TransactionError, TransactionResultError, Word, WORD_SIZE,
};
use miden_stdlib::StdLibrary;

use super::MidenLib;

pub mod memory;

mod inputs;
pub use inputs::ToTransactionKernelInputs;

mod outputs;
pub use outputs::{
    notes_try_from_elements, parse_final_account_stub, FINAL_ACCOUNT_HASH_WORD_IDX,
    OUTPUT_NOTES_COMMITMENT_WORD_IDX, TX_SCRIPT_ROOT_WORD_IDX,
};

// TRANSACTION KERNEL
// ================================================================================================

pub struct TransactionKernel;

impl TransactionKernel {
    // KERNEL SOURCE CODE
    // --------------------------------------------------------------------------------------------

    /// Returns MASM source code which encodes the transaction kernel system procedures.
    pub fn kernel() -> &'static str {
        include_str!("../../asm/miden/sat/kernel.masm")
    }

    /// Returns an AST of the transaction kernel executable program.
    ///
    /// # Errors
    /// Returns an error if deserialization of the binary fails.
    pub fn main() -> Result<ProgramAst, DeserializationError> {
        let kernel_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/transaction.masb"));
        ProgramAst::from_bytes(kernel_bytes)
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
            .expect("kernel is well formed")
    }

    // INPUT / OUTPUT STACK BUILDERS
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
        init_acct_hash: Option<Digest>,
        input_notes_hash: Digest,
        block_hash: Digest,
    ) -> StackInputs {
        let mut inputs: Vec<Felt> = Vec::with_capacity(13);
        inputs.extend(input_notes_hash);
        inputs.extend_from_slice(init_acct_hash.unwrap_or_default().as_elements());
        inputs.push(acct_id.into());
        inputs.extend_from_slice(block_hash.as_elements());
        StackInputs::new(inputs)
    }

    /// TODO: finish description
    ///
    /// Stack: [TXSR, CNC, FAH]
    ///
    /// Where:
    /// - TXSR is the transaction script root.
    /// - CNC is the commitment to the notes created by the transaction.
    /// - FAH is the final account hash of the account that the transaction is being
    ///   executed against.
    pub fn parse_output_stack(stack: &StackOutputs) -> (Digest, Digest, Digest) {
        // TODO: use constants
        let tx_script_root = stack.get_stack_word(0).expect("msg").into();
        let output_notes_hash = stack.get_stack_word(4).expect("msg").into();
        let final_account_hash = stack.get_stack_word(8).expect("msg").into();

        (final_account_hash, output_notes_hash, tx_script_root)
    }

    // ADVICE MAP EXTRACTORS
    // --------------------------------------------------------------------------------------------

    /// TODO: add comments
    pub fn parse_outputs(
        stack: &StackOutputs,
        adv_map: &AdviceMap,
    ) -> Result<TransactionOutputs, TransactionResultError> {
        let (final_acct_hash, output_notes_hash, _tx_script_root) = Self::parse_output_stack(stack);

        // --- parse final account state --------------------------------------
        let final_account_data: &[Word] = group_slice_elements(
            adv_map
                .get(final_acct_hash)
                .ok_or(TransactionResultError::FinalAccountDataNotFound)?,
        );
        let account = parse_final_account_stub(final_account_data)
            .map_err(TransactionResultError::FinalAccountStubDataInvalid)?;

        // --- parse output notes ---------------------------------------------

        let output_notes_data: &[Word] = group_slice_elements(
            adv_map
                .get(output_notes_hash)
                .ok_or(TransactionResultError::OutputNoteDataNotFound)?,
        );

        let mut output_notes = Vec::new();
        let mut output_note_ptr = 0;
        while output_note_ptr < output_notes_data.len() {
            let output_note = notes_try_from_elements(&output_notes_data[output_note_ptr..])
                .map_err(TransactionResultError::OutputNoteDataInvalid)?;
            output_notes.push(output_note);
            output_note_ptr += memory::NOTE_MEM_SIZE as usize;
        }

        let output_notes =
            OutputNotes::new(output_notes).map_err(TransactionResultError::OutputNotesError)?;
        if output_notes_hash != output_notes.commitment() {
            return Err(TransactionResultError::OutputNotesCommitmentInconsistent(
                output_notes_hash,
                output_notes.commitment(),
            ));
        }

        Ok(TransactionOutputs { account, output_notes })
    }

    // NOTE DATA BUILDER
    // --------------------------------------------------------------------------------------------

    /// TODO
    pub fn write_input_note_into(input_note: &InputNote, target: &mut Vec<Felt>) {
        let note = input_note.note();
        let proof = input_note.proof();

        // write the note info; 20 elements
        target.extend(note.serial_num());
        target.extend(*note.script().hash());
        target.extend(*note.inputs().hash());
        target.extend(*note.vault().hash());
        target.extend(Word::from(note.metadata()));

        // write asset vault; 4 * num_assets elements
        target.extend(note.vault().to_padded_assets());

        // write note location info; 10 elements
        target.push(proof.origin().block_num.into());
        target.extend(*proof.sub_hash());
        target.extend(*proof.note_root());
        target.push(proof.origin().node_index.value().into());
    }

    /// Returns a vectors of nullifiers read from the provided note data stream.
    ///
    /// Notes are expected to be arranged in the stream as follows:
    ///
    ///   [n, note_1_data, ... note_n_data]
    ///
    /// where n is the number of notes in the stream. Each note is expected to be arranged as
    /// follows:
    ///
    ///   [serial_num, script_hash, input_hash, vault_hash, metadata, asset_1 ... asset_k,
    ///    block_num, sub_hash, notes_root, note_index]
    ///
    /// Thus, the number of elements
    ///
    /// # Errors
    /// Returns an error if:
    /// - The stream does not contain at least one note.
    /// - The stream does not have enough data to read the specified number of notes.
    /// - The stream is not fully consumed after all notes have been processed.
    pub fn read_input_nullifiers_from(source: &[Felt]) -> Result<Vec<Nullifier>, TransactionError> {
        // extract the notes from the first fetch and instantiate a vector to hold nullifiers
        let num_notes = source[0].as_int();
        let mut nullifiers = Vec::with_capacity(num_notes as usize);

        // iterate over the notes and extract the nullifier and script root
        let mut note_ptr = 1;
        while note_ptr < source.len() {
            // make sure there is enough data to read (note data is well formed)
            if note_ptr + 5 * WORD_SIZE > source.len() {
                return Err(TransactionError::InvalidInputNoteDataLength);
            }

            // compute the nullifier and extract script root and number of assets
            let (nullifier, num_assets) = extract_note_data(&source[note_ptr..]);

            // push the [ConsumedNoteInfo] to the vector
            nullifiers.push(nullifier.into());

            // round up the number of assets to the next multiple of 2 to account for asset padding
            let num_assets = (num_assets + 1) & !1;

            // increment note pointer
            note_ptr += (num_assets as usize * WORD_SIZE) + 30;
        }

        Ok(nullifiers)
    }
}

// HELPERS
// ================================================================================================

/// Extracts and returns the nullifier and the number of assets from the provided note data.
///
/// Expects the note data to be organized as follows:
/// [CN_SN, CN_SR, CN_IR, CN_VR, CN_M]
///
/// - CN_SN is the serial number of the consumed note.
/// - CN_SR is the script root of the consumed note.
/// - CN_IR is the inputs root of the consumed note.
/// - CN_VR is the vault root of the consumed note.
/// - CN1_M is the metadata of the consumed note.
fn extract_note_data(note_data: &[Felt]) -> (Digest, u64) {
    // compute the nullifier
    let nullifier = Hasher::hash_elements(&note_data[..4 * WORD_SIZE]);

    // extract the number of assets
    let num_assets = note_data[4 * WORD_SIZE].as_int();

    (nullifier, num_assets)
}

// ADVICE MAP
// ================================================================================================

pub struct AdviceMap(BTreeMap<[u8; 32], Vec<Felt>>);

impl AdviceMap {
    pub fn get(&self, key: Digest) -> Option<&Vec<Felt>> {
        self.0.get(&key.as_bytes())
    }
}

impl From<BTreeMap<[u8; 32], Vec<Felt>>> for AdviceMap {
    fn from(value: BTreeMap<[u8; 32], Vec<Felt>>) -> Self {
        Self(value)
    }
}
