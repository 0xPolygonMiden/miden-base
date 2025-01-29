use miden_objects::{
    batch::{ProposedBatch, ProvenBatch},
    BatchError,
};

// LOCAL BATCH PROVER
// ================================================================================================

/// A local prover for transaction batches, turning a [`ProposedBatch`] into a [`ProvenBatch`].
pub struct LocalBatchProver {}

impl LocalBatchProver {
    /// Attempts to prove the [`ProposedBatch`] into a [`ProvenBatch`].
    /// TODO
    pub fn prove(proposed_batch: ProposedBatch) -> Result<ProvenBatch, BatchError> {
        let (
            _transactions,
            _block_header,
            _block_chain,
            _authenticatable_unauthenticated_notes,
            id,
            updated_accounts,
            input_notes,
            output_notes_smt,
            output_notes,
            batch_expiration_block_num,
        ) = proposed_batch.into_parts();

        Ok(ProvenBatch::new(
            id,
            updated_accounts,
            input_notes,
            output_notes_smt,
            output_notes,
            batch_expiration_block_num,
        ))
    }
}
