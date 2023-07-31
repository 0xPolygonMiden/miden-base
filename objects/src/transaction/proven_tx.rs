use super::{
    AccountId, ConsumedNoteInfo, Digest, Felt, Hasher, NoteEnvelope, StackInputs, StackOutputs,
    Vec, Word,
};
use crypto::{WORD_SIZE, ZERO};
use miden_core::stack::STACK_TOP_SIZE;
use miden_verifier::ExecutionProof;

/// Resultant object of executing and proving a transaction. It contains the minimal
/// amount of data needed to verify that the transaction was executed correctly.
/// Contains:
/// - account_id: the account that the transaction was executed against.
/// - initial_account_hash: the hash of the account before the transaction was executed.
/// - final_account_hash: the hash of the account after the transaction was executed.
/// - consumed_notes: a list of consumed notes.
/// - created_notes: a list of created notes.
/// - tx_script_root: the script root of the transaction.
/// - block_ref: the block hash of the last known block at the time the transaction was executed.
/// - proof: the proof of the transaction.
pub struct ProvenTransaction {
    account_id: AccountId,
    initial_account_hash: Digest,
    final_account_hash: Digest,
    consumed_notes: Vec<ConsumedNoteInfo>,
    created_notes: Vec<NoteEnvelope>,
    tx_script_root: Option<Digest>,
    program_hash: Digest,
    block_ref: Digest,
    proof: ExecutionProof,
}

impl ProvenTransaction {
    #[allow(clippy::too_many_arguments)]
    /// Creates a new ProvenTransaction object.
    pub fn new(
        account_id: AccountId,
        initial_account_hash: Digest,
        final_account_hash: Digest,
        consumed_notes: Vec<ConsumedNoteInfo>,
        created_notes: Vec<NoteEnvelope>,
        tx_script_root: Option<Digest>,
        program_hash: Digest,
        block_ref: Digest,
        proof: ExecutionProof,
    ) -> Self {
        Self {
            account_id,
            initial_account_hash,
            final_account_hash,
            consumed_notes,
            created_notes,
            tx_script_root,
            program_hash,
            block_ref,
            proof,
        }
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------
    /// Returns the account ID.
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Returns the initial account hash.
    pub fn initial_account_hash(&self) -> Digest {
        self.initial_account_hash
    }

    /// Returns the final account hash.
    pub fn final_account_hash(&self) -> Digest {
        self.final_account_hash
    }

    /// Returns the consumed notes.
    pub fn consumed_notes(&self) -> &[ConsumedNoteInfo] {
        &self.consumed_notes
    }

    /// Returns the created notes.
    pub fn created_notes(&self) -> &[NoteEnvelope] {
        &self.created_notes
    }
    /// Returns the script root of the transaction.
    pub fn tx_script_root(&self) -> Option<Digest> {
        self.tx_script_root
    }

    /// Returns the transaction program info.
    pub fn program_hash(&self) -> Digest {
        self.program_hash
    }

    /// Returns the proof of the transaction.
    pub fn proof(&self) -> &ExecutionProof {
        &self.proof
    }

    /// Returns the block reference the transaction was executed against.
    pub fn block_ref(&self) -> Digest {
        self.block_ref
    }

    /// Returns the consumed notes commitment.
    pub fn compute_consumed_notes_hash(&self) -> Digest {
        let mut elements: Vec<Felt> = Vec::with_capacity(self.consumed_notes.len() * 8);
        for note in self.consumed_notes.iter() {
            elements.extend_from_slice(note.nullifier().as_elements());
            elements.extend_from_slice(note.script_root().as_elements());
        }
        Hasher::hash_elements(&elements)
    }

    /// Returns the created notes commitment.
    pub fn compute_created_notes_commitment(&self) -> Digest {
        let mut elements: Vec<Felt> = Vec::with_capacity(self.created_notes.len() * 8);
        for note in self.created_notes.iter() {
            elements.extend_from_slice(note.note_hash().as_elements());
            elements.extend_from_slice(&Word::from(note.metadata()));
        }
        Hasher::hash_elements(&elements)
    }

    /// Returns the stack inputs for the transaction.
    pub fn build_stack_inputs(&self) -> StackInputs {
        let mut stack_inputs: Vec<Felt> = Vec::with_capacity(13);
        stack_inputs.extend_from_slice(self.compute_consumed_notes_hash().as_elements());
        stack_inputs.extend_from_slice(self.initial_account_hash.as_elements());
        stack_inputs.push(*self.account_id);
        stack_inputs.extend_from_slice(self.block_ref.as_elements());
        StackInputs::new(stack_inputs)
    }

    /// Returns the stack outputs for the transaction.
    pub fn build_stack_outputs(&self) -> StackOutputs {
        let mut stack_outputs: Vec<Felt> = vec![ZERO; STACK_TOP_SIZE];
        stack_outputs[STACK_TOP_SIZE - WORD_SIZE..]
            .copy_from_slice(self.compute_created_notes_commitment().as_elements());
        stack_outputs[STACK_TOP_SIZE - (2 * WORD_SIZE)..STACK_TOP_SIZE - WORD_SIZE]
            .copy_from_slice(self.final_account_hash.as_elements());
        stack_outputs.reverse();
        StackOutputs::from_elements(stack_outputs, Default::default())
            .expect("StackOutputs are valid")
    }
}
