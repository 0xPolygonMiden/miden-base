use super::{Digest, Hasher, TransactionCompiler, TransactionVerifierError};
use core::ops::Range;
use miden_lib::outputs::{
    CREATED_NOTES_COMMITMENT_WORD_IDX, FINAL_ACCOUNT_HASH_WORD_IDX, TX_SCRIPT_ROOT_WORD_IDX,
};
use miden_objects::{
    notes::{NoteEnvelope, Nullifier},
    transaction::ProvenTransaction,
    Felt, Word, WORD_SIZE, ZERO,
};
use miden_verifier::verify;
use vm_core::{stack::STACK_TOP_SIZE, ProgramInfo, StackInputs, StackOutputs};

/// The [TransactionVerifier] is used to verify a [ProvenTransaction].
///
/// The [TransactionVerifier] contains a [ProgramInfo] object which is associated with the
/// transaction kernel program.  The `proof_security_level` specifies the minimum security
/// level that the transaction proof must have in order to be considered valid.
pub struct TransactionVerifier {
    tx_program_info: ProgramInfo,
    proof_security_level: u32,
}

impl TransactionVerifier {
    /// Creates a new [TransactionVerifier] object.
    pub fn new(proof_security_level: u32) -> Self {
        // TODO: create program info at build time?
        let tx_program_info = TransactionCompiler::new().build_program_info();
        Self {
            tx_program_info,
            proof_security_level,
        }
    }

    /// Verifies the provided [ProvenTransaction] against the kernel.
    ///
    /// # Errors
    /// - if transaction verification fails.
    /// - if the proof security level is insufficient.
    pub fn verify(&self, transaction: ProvenTransaction) -> Result<(), TransactionVerifierError> {
        let proof_security_level = verify(
            self.tx_program_info.clone(),
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
    fn compute_consumed_notes_hash(consumed_notes: &[Nullifier]) -> Digest {
        let mut elements: Vec<Felt> = Vec::with_capacity(consumed_notes.len() * 8);
        for nullifier in consumed_notes.iter() {
            elements.extend_from_slice(nullifier.inner().as_elements());
            elements.extend_from_slice(&Word::default());
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
        stack_inputs.push(transaction.account_id().into());
        stack_inputs.extend_from_slice(transaction.block_ref().as_elements());
        StackInputs::new(stack_inputs)
    }

    /// Returns the stack outputs for the transaction.
    fn build_stack_outputs(transaction: &ProvenTransaction) -> StackOutputs {
        const TX_SCRIPT_ROOT_RANGE: Range<usize> = Range {
            start: STACK_TOP_SIZE - ((TX_SCRIPT_ROOT_WORD_IDX + 1) * WORD_SIZE),
            end: STACK_TOP_SIZE - (TX_SCRIPT_ROOT_WORD_IDX * WORD_SIZE),
        };
        const CREATED_NOTES_COMMITMENT_RANGE: Range<usize> = Range {
            start: STACK_TOP_SIZE - ((CREATED_NOTES_COMMITMENT_WORD_IDX + 1) * WORD_SIZE),
            end: STACK_TOP_SIZE - (CREATED_NOTES_COMMITMENT_WORD_IDX * WORD_SIZE),
        };
        const FINAL_ACCOUNT_HASH_RANGE: Range<usize> = Range {
            start: STACK_TOP_SIZE - ((FINAL_ACCOUNT_HASH_WORD_IDX + 1) * WORD_SIZE),
            end: STACK_TOP_SIZE - (FINAL_ACCOUNT_HASH_WORD_IDX * WORD_SIZE),
        };

        let mut stack_outputs: Vec<Felt> = vec![ZERO; STACK_TOP_SIZE];
        stack_outputs[TX_SCRIPT_ROOT_RANGE]
            .copy_from_slice(transaction.tx_script_root().unwrap_or_default().as_elements());
        stack_outputs[CREATED_NOTES_COMMITMENT_RANGE].copy_from_slice(
            Self::compute_created_notes_commitment(transaction.created_notes()).as_elements(),
        );
        stack_outputs[FINAL_ACCOUNT_HASH_RANGE]
            .copy_from_slice(transaction.final_account_hash().as_elements());
        stack_outputs.reverse();
        StackOutputs::from_elements(stack_outputs, Default::default())
            .expect("StackOutputs are valid")
    }
}
