use std::{collections::BTreeMap, vec::Vec};

use miden_crypto::merkle::{LeafIndex, PartialMerkleTree};
use miden_objects::{
    account::AccountId,
    block::{
        AccountUpdateWitness, BlockHeader, BlockNumber, NullifierWitness, PartialNullifierTree,
        ProposedBlock, ProvenBlock,
    },
    note::Nullifier,
    transaction::ChainMmr,
    Digest, Word,
};

use crate::errors::ProvenBlockError;

// LOCAL BLOCK PROVER
// ================================================================================================

/// A local prover for blocks, proving a [`ProposedBlock`] and returning a [`Block`].
pub struct LocalBlockProver {}

impl LocalBlockProver {
    /// Creates a new [`LocalBlockProver`] instance.
    pub fn new(_proof_security_level: u32) -> Self {
        // TODO: This will eventually take the security level as a parameter, but until we verify
        // batches it is ignored.
        Self {}
    }

    /// TODO: Document.
    /// TODO: Expose under testing feature.
    pub fn prove_without_verification(
        &self,
        proposed_block: ProposedBlock,
    ) -> Result<ProvenBlock, ProvenBlockError> {
        let block_num = proposed_block.block_num();
        let tx_hash = BlockHeader::compute_tx_commitment(proposed_block.affected_accounts());

        let timestamp = proposed_block.timestamp();
        let (
            _batches,
            account_updated_witnesses,
            output_note_batches,
            block_note_tree,
            created_nullifiers,
            chain_mmr,
            prev_block_header,
        ) = proposed_block.into_parts();

        let prev_block_commitment = prev_block_header.hash();
        let note_root = block_note_tree.root();

        let new_nullifier_root =
            compute_nullifier_root(created_nullifiers, &prev_block_header, block_num)?;

        let new_chain_root = compute_chain_root(chain_mmr, prev_block_header);

        let new_account_root =
            compute_account_root(account_updated_witnesses).expect("TODO: error");

        // TODO: Where is this defined?
        let version = 0;
        // TODO: How should we compute this? Which kernel do we mean (tx, batch, block)? Should we
        // rename it to indicate that?
        let kernel_root = Digest::default();

        // For now, we're not actually proving the block.
        let proof_hash = Digest::default();

        let header = BlockHeader::new(
            version,
            prev_block_commitment,
            block_num,
            new_chain_root,
            new_account_root,
            new_nullifier_root,
            note_root,
            tx_hash,
            kernel_root,
            proof_hash,
            timestamp,
        );

        let updated_accounts = vec![];
        let created_nullifiers = vec![]; //created_nullifiers.into_keys().collect();

        let proven_block = ProvenBlock::new_unchecked(
            header,
            updated_accounts,
            output_note_batches,
            created_nullifiers,
        );

        Ok(proven_block)
    }
}

// TODO: Describe assumptions about inputs (completeness).

/// Validates that the nullifiers returned from the store are the same the produced nullifiers
/// in the batches. Note that validation that the value of the nullifiers is `0` will be
/// done in MASM.
fn compute_nullifier_root(
    created_nullifiers: BTreeMap<Nullifier, NullifierWitness>,
    prev_block_header: &BlockHeader,
    block_num: BlockNumber,
) -> Result<Digest, ProvenBlockError> {
    let mut partial_nullifier_tree = PartialNullifierTree::new();

    // Due to the guarantees of ProposedBlock we can safely assume that each nullifier is mapped to
    // its corresponding nullifier witness, so we don't have to check again whether they match.
    for witness in created_nullifiers.values() {
        partial_nullifier_tree
            .add_nullifier_witness(witness.clone())
            .map_err(|source| ProvenBlockError::NullifierWitnessRootMismatch { source })?;
    }

    debug_assert_eq!(
        partial_nullifier_tree.root(),
        prev_block_header.nullifier_root(),
        "partial nullifier tree root should match nullifier root of previous block header as validated in the loop"
    );

    for nullifier in created_nullifiers.into_keys() {
        // SAFETY: As mentioned above, we can safely assume that each nullifier's witness was added
        // and every nullifier should be tracked by the partial tree and therefore updatable.
        partial_nullifier_tree.mark_spent(nullifier, block_num).expect(
            "we should have previously added this nullifier's merkle path to the partial tree",
        );
    }

    Ok(partial_nullifier_tree.root())
}

/// Adds the commitment of the previous block header to the chain MMR to compute the new chain root.
fn compute_chain_root(mut chain_mmr: ChainMmr, prev_block_header: BlockHeader) -> Digest {
    // SAFETY: This does not panic as long as the block header we're adding is the next one in the
    // chain which is validated as part of constructing a `ProposedBlock`.
    chain_mmr.add_block(prev_block_header, true);
    chain_mmr.peaks().hash_peaks()
}

/// Computes the new account tree root after the given updates.
///
/// It uses a PartialMerkleTree for now while we use a SimpleSmt for the account tree. Once that is
/// updated to an Smt, we can use PartialSmt instead.
fn compute_account_root(
    mut updated_accounts: Vec<(AccountId, AccountUpdateWitness)>,
) -> Result<Digest, ProvenBlockError> {
    let mut partial_account_tree = PartialMerkleTree::new();

    // First reconstruct the current account tree from the provided merkle paths.
    for (account_id, witness) in updated_accounts.iter_mut() {
        let account_leaf_index = LeafIndex::from(*account_id);
        // Shouldn't the value in PartialMerkleTree::add_path be a Word instead of a Digest?
        partial_account_tree
            .add_path(
                account_leaf_index.value(),
                witness.initial_state_commitment(),
                // We don't need the merkle path later, so we can take it out.
                core::mem::take(witness.initial_state_proof_mut()),
            )
            .map_err(|source| ProvenBlockError::AccountWitnessRootMismatch {
                account_id: *account_id,
                source,
            })?;
    }

    // Second, update the account tree by inserting the new final account state commitments to
    // compute the new root of the account tree.
    for (account_id, witness) in updated_accounts {
        let account_leaf_index = LeafIndex::from(account_id);
        partial_account_tree
            .update_leaf(account_leaf_index.value(), Word::from(witness.final_state_commitment()))
            .expect("every account leaf should have been inserted in the first loop");
    }

    Ok(partial_account_tree.root())
}
