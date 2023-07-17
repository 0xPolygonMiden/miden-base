use super::{
    AccountId, ConsumedNoteInfo, CreatedNoteInfo, Digest, Felt, Hasher, StackInputs, StackOutputs,
    Vec, Word, ZERO,
};
use crypto::WORD_SIZE;
use miden_core::{stack::STACK_TOP_SIZE, ProgramInfo};
use miden_verifier::{verify, ExecutionProof, VerificationError};

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
#[derive(Debug)]
pub struct ProvenTransaction {
    account_id: AccountId,
    initial_account_hash: Digest,
    final_account_hash: Digest,
    consumed_notes: Vec<ConsumedNoteInfo>,
    created_notes: Vec<CreatedNoteInfo>,
    tx_script_root: Option<Digest>,
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
        created_notes: Vec<CreatedNoteInfo>,
        tx_script_root: Option<Digest>,
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
            block_ref,
            proof,
        }
    }

    /// Verify the transaction using the provided data and proof.
    /// Returns the security level of the proof if the specified program was executed correctly against
    /// the specified inputs and outputs.
    ///
    /// # Errors
    /// Returns an error if the provided proof does not prove a correct execution of the program.
    pub fn verify(&self) -> Result<u32, VerificationError> {
        verify(
            self.tx_program_info(),
            self.stack_inputs(),
            self.stack_outputs(),
            self.proof.clone(),
        )
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

    /// Returns the consumed notes commitment.
    pub fn consumed_notes_hash(&self) -> Digest {
        let mut elements: Vec<Felt> = Vec::with_capacity(self.consumed_notes.len() * 8);
        for note in self.consumed_notes.iter() {
            elements.extend_from_slice(note.nullifier().as_elements());
            elements.extend_from_slice(note.script_root().as_elements());
        }
        Hasher::hash_elements(&elements)
    }

    /// Returns the created notes.
    pub fn created_notes(&self) -> &[CreatedNoteInfo] {
        &self.created_notes
    }

    /// Returns the created notes commitment.
    pub fn created_notes_commitment(&self) -> Digest {
        let mut elements: Vec<Felt> = Vec::with_capacity(self.created_notes.len() * 8);
        for note in self.created_notes.iter() {
            elements.extend_from_slice(note.note_hash().as_elements());
            elements.extend_from_slice(&Word::from(note.metadata()));
        }
        Hasher::hash_elements(&elements)
    }

    /// Returns the transaction program info.
    pub fn tx_program_info(&self) -> ProgramInfo {
        todo!()
    }

    /// Returns the stack inputs for the transaction.
    pub fn stack_inputs(&self) -> StackInputs {
        let mut stack_inputs: Vec<Felt> = Vec::with_capacity(13);
        stack_inputs.extend_from_slice(self.consumed_notes_hash().as_elements());
        stack_inputs.extend_from_slice(self.initial_account_hash.as_elements());
        stack_inputs.push(*self.account_id);
        stack_inputs.extend_from_slice(self.block_ref.as_elements());
        StackInputs::new(stack_inputs)
    }

    /// Returns the stack outputs for the transaction.
    pub fn stack_outputs(&self) -> StackOutputs {
        let mut stack_outputs: Vec<Felt> = vec![ZERO; STACK_TOP_SIZE];
        stack_outputs[STACK_TOP_SIZE - WORD_SIZE..]
            .copy_from_slice(self.created_notes_commitment().as_elements());
        stack_outputs[STACK_TOP_SIZE - (2 * WORD_SIZE)..STACK_TOP_SIZE - WORD_SIZE]
            .copy_from_slice(self.final_account_hash.as_elements());
        stack_outputs.reverse();
        StackOutputs::from_elements(stack_outputs, Default::default())
    }

    /// Returns the script root of the transaction.
    pub fn tx_script_root(&self) -> Option<Digest> {
        self.tx_script_root
    }

    /// Returns the transaction proof.
    pub fn proof(&self) -> &ExecutionProof {
        &self.proof
    }
}
