use super::{Digest, Hasher, TransactionComplier, TransactionVerifierError};
use crypto::{WORD_SIZE, ZERO};
use miden_core::{stack::STACK_TOP_SIZE, Felt, StackInputs, StackOutputs, Word};
use miden_objects::{
    notes::NoteEnvelope,
    transaction::{ConsumedNoteInfo, ProvenTransaction},
};
use miden_verifier::verify;

/// The [TransactionVerifier] is used to verify a [ProvenTransaction].
///
/// The [TransactionVerifier] contains a [TransactionComplier] object which we use to construct
/// the transaction program associated with a transaction.  The `proof_security_level` specifies
/// the minimum security level that the transaction proof must have in order to be considered
/// valid.
pub struct TransactionVerifier {
    compiler: TransactionComplier,
    proof_security_level: u32,
}

impl TransactionVerifier {
    /// Creates a new [TransactionVerifier] object.
    pub fn new(proof_security_level: u32) -> Self {
        let compiler = TransactionComplier::new();
        Self {
            compiler,
            proof_security_level,
        }
    }

    /// Verifies the provided [ProvenTransaction] against the kernel.
    pub fn verify(&self, transaction: ProvenTransaction) -> Result<(), TransactionVerifierError> {
        let consumed_notes_hashes =
            transaction.consumed_notes().iter().map(|x| x.script_root()).collect();
        let program_info = self
            .compiler
            .build_program_info(consumed_notes_hashes, transaction.tx_script_root());

        let proof_security_level = verify(
            program_info,
            Self::build_stack_inputs(&transaction),
            Self::build_stack_outputs(&transaction),
            transaction.proof().clone(),
        )
        .map_err(TransactionVerifierError::TransactionVerificationFailed)?;

        if proof_security_level < self.proof_security_level {
            return Err(TransactionVerifierError::InsufficientProofSecurityLevel(
                proof_security_level,
                self.proof_security_level,
            ));
        }

        Ok(())
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------
    /// Returns the consumed notes commitment.
    fn compute_consumed_notes_hash(consumed_notes: &[ConsumedNoteInfo]) -> Digest {
        let mut elements: Vec<Felt> = Vec::with_capacity(consumed_notes.len() * 8);
        for note in consumed_notes.iter() {
            elements.extend_from_slice(note.nullifier().as_elements());
            elements.extend_from_slice(note.script_root().as_elements());
        }
        Hasher::hash_elements(&elements)
    }

    /// Returns the created notes commitment.
    fn compute_created_notes_commitment(created_notes: &[NoteEnvelope]) -> Digest {
        let mut elements: Vec<Felt> = Vec::with_capacity(created_notes.len() * 8);
        for note in created_notes.iter() {
            elements.extend_from_slice(note.note_hash().as_elements());
            elements.extend_from_slice(&Word::from(note.metadata()));
        }
        Hasher::hash_elements(&elements)
    }

    /// Returns the stack inputs for the transaction.
    fn build_stack_inputs(transaction: &ProvenTransaction) -> StackInputs {
        let mut stack_inputs: Vec<Felt> = Vec::with_capacity(13);
        stack_inputs.extend_from_slice(
            Self::compute_consumed_notes_hash(transaction.consumed_notes()).as_elements(),
        );
        stack_inputs.extend_from_slice(transaction.initial_account_hash().as_elements());
        stack_inputs.push(*transaction.account_id());
        stack_inputs.extend_from_slice(transaction.block_ref().as_elements());
        StackInputs::new(stack_inputs)
    }

    /// Returns the stack outputs for the transaction.
    fn build_stack_outputs(transaction: &ProvenTransaction) -> StackOutputs {
        let mut stack_outputs: Vec<Felt> = vec![ZERO; STACK_TOP_SIZE];
        stack_outputs[STACK_TOP_SIZE - WORD_SIZE..].copy_from_slice(
            Self::compute_created_notes_commitment(transaction.created_notes()).as_elements(),
        );
        stack_outputs[STACK_TOP_SIZE - (2 * WORD_SIZE)..STACK_TOP_SIZE - WORD_SIZE]
            .copy_from_slice(transaction.final_account_hash().as_elements());
        stack_outputs.reverse();
        StackOutputs::from_elements(stack_outputs, Default::default())
            .expect("StackOutputs are valid")
    }
}
