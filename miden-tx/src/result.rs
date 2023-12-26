use miden_lib::{
    memory::NOTE_MEM_SIZE,
    notes::notes_try_from_elements,
    outputs::{CREATED_NOTES_COMMITMENT_WORD_IDX, FINAL_ACCOUNT_HASH_WORD_IDX},
    transaction::parse_final_account_stub,
};
use miden_objects::{
    crypto::merkle::MerkleStore,
    transaction::{FinalAccountStub, OutputNote, OutputNotes},
    utils::collections::{BTreeMap, Vec},
    Digest, Felt, TransactionResultError, Word, WORD_SIZE,
};
use vm_core::utils::group_slice_elements;
use vm_processor::StackOutputs;

/// A trait that defines the interface for extracting objects from the result of a VM execution.
pub trait TryFromVmResult: Sized {
    type Error;

    /// Tries to create an object from the provided stack outputs and advice provider components.
    fn try_from_vm_result(
        stack_outputs: &StackOutputs,
        advice_stack: &[Felt],
        advice_map: &BTreeMap<[u8; 32], Vec<Felt>>,
        merkle_store: &MerkleStore,
    ) -> Result<Self, Self::Error>;
}

impl TryFromVmResult for OutputNotes {
    type Error = TransactionResultError;

    fn try_from_vm_result(
        stack_outputs: &StackOutputs,
        _advice_stack: &[Felt],
        advice_map: &BTreeMap<[u8; 32], Vec<Felt>>,
        _merkle_store: &MerkleStore,
    ) -> Result<Self, Self::Error> {
        let created_notes_commitment: Word =
            stack_outputs.stack()[CREATED_NOTES_COMMITMENT_WORD_IDX * WORD_SIZE
                ..(CREATED_NOTES_COMMITMENT_WORD_IDX + 1) * WORD_SIZE]
                .iter()
                .rev()
                .map(|x| Felt::from(*x))
                .collect::<Vec<_>>()
                .try_into()
                .expect("word size is correct");
        let created_notes_commitment: Digest = created_notes_commitment.into();

        let created_notes_data = group_slice_elements::<Felt, WORD_SIZE>(
            advice_map
                .get(&created_notes_commitment.as_bytes())
                .ok_or(TransactionResultError::OutputNoteDataNotFound)?,
        );

        let mut created_notes = Vec::new();
        let mut created_note_ptr = 0;
        while created_note_ptr < created_notes_data.len() {
            let note_stub: OutputNote =
                notes_try_from_elements(&created_notes_data[created_note_ptr..])
                    .map_err(TransactionResultError::OutputNoteDataInvalid)?;
            created_notes.push(note_stub);
            created_note_ptr += NOTE_MEM_SIZE as usize;
        }

        let created_notes = Self::new(created_notes)?;
        if created_notes_commitment != created_notes.commitment() {
            return Err(TransactionResultError::OutputNotesCommitmentInconsistent(
                created_notes_commitment,
                created_notes.commitment(),
            ));
        }

        Ok(created_notes)
    }
}

impl TryFromVmResult for FinalAccountStub {
    type Error = TransactionResultError;

    fn try_from_vm_result(
        stack_outputs: &StackOutputs,
        _advice_stack: &[Felt],
        advice_map: &BTreeMap<[u8; 32], Vec<Felt>>,
        _merkle_store: &MerkleStore,
    ) -> Result<Self, Self::Error> {
        let final_account_hash: Word =
            stack_outputs.stack()[FINAL_ACCOUNT_HASH_WORD_IDX * WORD_SIZE
                ..(FINAL_ACCOUNT_HASH_WORD_IDX + 1) * WORD_SIZE]
                .iter()
                .rev()
                .map(|x| Felt::from(*x))
                .collect::<Vec<_>>()
                .try_into()
                .expect("word size is correct");
        let final_account_hash: Digest = final_account_hash.into();

        // extract final account data from the advice map
        let final_account_data = group_slice_elements::<Felt, WORD_SIZE>(
            advice_map
                .get(&final_account_hash.as_bytes())
                .ok_or(TransactionResultError::FinalAccountDataNotFound)?,
        );
        let stub = parse_final_account_stub(final_account_data)
            .map_err(TransactionResultError::FinalAccountStubDataInvalid)?;

        Ok(Self(stub))
    }
}
