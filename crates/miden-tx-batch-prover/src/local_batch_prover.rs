use miden_objects::batch::{ProposedBatch, ProvenBatch};
use miden_tx::TransactionVerifier;

use crate::errors::ProvenBatchError;

// LOCAL BATCH PROVER
// ================================================================================================

/// A local prover for transaction batches, proving the transactions in a [`ProposedBatch`] and
/// returning a [`ProvenBatch`].
pub struct LocalBatchProver {
    proof_security_level: u32,
}

impl LocalBatchProver {
    /// Creates a new [`LocalBatchProver`] instance.
    pub fn new(proof_security_level: u32) -> Self {
        Self { proof_security_level }
    }

    /// Attempts to prove the [`ProposedBatch`] into a [`ProvenBatch`].
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - a proof of any transaction in the batch fails to verify.
    pub fn prove(&self, proposed_batch: ProposedBatch) -> Result<ProvenBatch, ProvenBatchError> {
        let (
            transactions,
            block_header,
            _block_chain,
            _authenticatable_unauthenticated_notes,
            id,
            updated_accounts,
            input_notes,
            output_notes,
            batch_expiration_block_num,
        ) = proposed_batch.into_parts();

        let verifier = TransactionVerifier::new(self.proof_security_level);

        for tx in transactions {
            verifier.verify(&tx).map_err(|source| {
                ProvenBatchError::TransactionVerificationFailed { transaction_id: tx.id(), source }
            })?;
        }

        Ok(ProvenBatch::new_unchecked(
            id,
            block_header.hash(),
            block_header.block_num(),
            updated_accounts,
            input_notes,
            output_notes,
            batch_expiration_block_num,
        ))
    }
}
