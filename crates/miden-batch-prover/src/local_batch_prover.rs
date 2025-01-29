use miden_objects::{
    account::AccountId,
    batch::{ProposedBatch, ProvenBatch},
    block::{BlockHeader, BlockNumber},
    note::{NoteHeader, NoteInclusionProof},
    transaction::OutputNote,
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

// HELPER FUNCTIONS
// ================================================================================================

/// Validates whether the provided unauthenticated note belongs to the note tree of the specified
/// block header.
// TODO: Remove allow once used.
#[allow(dead_code)]
fn authenticate_unauthenticated_note(
    note_header: &NoteHeader,
    proof: &NoteInclusionProof,
    block_header: &BlockHeader,
) -> Result<(), BatchError> {
    let note_index = proof.location().node_index_in_block().into();
    let note_hash = note_header.hash();
    proof
        .note_path()
        .verify(note_index, note_hash, &block_header.note_root())
        .map_err(|_| BatchError::UnauthenticatedNoteAuthenticationFailed {
            note_id: note_header.id(),
            block_num: proof.location().block_num(),
        })
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;

    use miden_crypto::merkle::{MerklePath, MmrPeaks, PartialMmr};
    use miden_lib::{account::wallets::BasicWallet, transaction::TransactionKernel};
    use miden_objects::{
        account::{Account, AccountBuilder},
        note::{Note, NoteInclusionProof},
        testing::{account_id::AccountIdBuilder, note::NoteBuilder},
        transaction::{ChainMmr, InputNote},
        BatchAccountUpdateError,
    };
    use rand::{rngs::SmallRng, SeedableRng};
    use vm_core::assert_matches;
    use vm_processor::Digest;

    use super::*;
    use crate::testing::MockProvenTxBuilder;

    fn mock_chain_mmr() -> ChainMmr {
        ChainMmr::new(PartialMmr::from_peaks(MmrPeaks::new(0, vec![]).unwrap()), vec![]).unwrap()
    }

    fn mock_block_header(block_num: u32) -> BlockHeader {
        let chain_root = mock_chain_mmr().peaks().hash_peaks();
        BlockHeader::mock(block_num, Some(chain_root), None, &[], Digest::default())
    }

    fn mock_account_id(num: u8) -> AccountId {
        AccountIdBuilder::new().build_with_rng(&mut SmallRng::from_seed([num; 32]))
    }

    fn mock_wallet_account(num: u8) -> Account {
        AccountBuilder::new([num; 32])
            .with_component(BasicWallet)
            .build_existing()
            .unwrap()
    }

    pub fn mock_note(num: u8) -> Note {
        let sender = mock_account_id(num);
        NoteBuilder::new(sender, SmallRng::from_seed([num; 32]))
            .build(&TransactionKernel::assembler().with_debug_mode(true))
            .unwrap()
    }

    pub fn mock_output_note(num: u8) -> OutputNote {
        OutputNote::Full(mock_note(num))
    }

    pub fn mock_proof(node_index: u16) -> NoteInclusionProof {
        NoteInclusionProof::new(BlockNumber::from(0), node_index, MerklePath::new(vec![])).unwrap()
    }

    /*


    */
}
