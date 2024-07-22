use alloc::vec::Vec;

use miden_lib::transaction::TransactionKernel;
use miden_objects::{transaction::ProvenTransaction, vm::ProgramInfo, Digest, Felt, Hasher};
use miden_verifier::verify;

use super::TransactionVerifierError;

// TRANSACTION VERIFIER
// ================================================================================================

/// The [TransactionVerifier] is used to verify  [ProvenTransaction]s.
///
/// The [TransactionVerifier] contains a [ProgramInfo] object which is associated with the
/// transaction kernel program.  The `proof_security_level` specifies the minimum security
/// level that the transaction proof must have in order to be considered valid.
pub struct TransactionVerifier {
    tx_program_info: ProgramInfo,
    proof_security_level: u32,
}

impl TransactionVerifier {
    /// Returns a new [TransactionVerifier] instantiated with the specified security level.
    pub fn new(proof_security_level: u32) -> Self {
        let tx_program_info = TransactionKernel::program_info();
        Self { tx_program_info, proof_security_level }
    }

    /// Verifies the provided [ProvenTransaction] against the transaction kernel.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Transaction verification fails.
    /// - The security level of the verified proof is insufficient.
    pub fn verify(&self, transaction: ProvenTransaction) -> Result<(), TransactionVerifierError> {
        // compute the kernel hash
        let kernel = self.tx_program_info.kernel();
        // we need to get &[Felt] from &[Digest]
        let kernel_procs_as_felts = Digest::digests_as_elements(kernel.proc_hashes().into_iter())
            .cloned()
            .collect::<Vec<Felt>>();
        let kernel_hash = Hasher::hash_elements(&kernel_procs_as_felts);

        // build stack inputs and outputs
        let stack_inputs = TransactionKernel::build_input_stack(
            transaction.account_id(),
            transaction.account_update().init_state_hash(),
            transaction.input_notes().commitment(),
            transaction.block_ref(),
            (kernel.proc_hashes().len(), kernel_hash)
        );
        let stack_outputs = TransactionKernel::build_output_stack(
            transaction.account_update().final_state_hash(),
            transaction.output_notes().commitment(),
        );

        // verify transaction proof
        let proof_security_level = verify(
            self.tx_program_info.clone(),
            stack_inputs,
            stack_outputs,
            transaction.proof().clone(),
        )
        .map_err(TransactionVerifierError::TransactionVerificationFailed)?;

        // check security level
        if proof_security_level < self.proof_security_level {
            return Err(TransactionVerifierError::InsufficientProofSecurityLevel(
                proof_security_level,
                self.proof_security_level,
            ));
        }

        Ok(())
    }
}
