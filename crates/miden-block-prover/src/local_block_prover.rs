use std::{collections::BTreeMap, vec::Vec};

use miden_crypto::merkle::{LeafIndex, PartialMerkleTree};
use miden_objects::{
    account::AccountId,
    block::{
        AccountUpdateWitness, BlockAccountUpdate, BlockHeader, BlockNumber, NullifierWitness,
        PartialNullifierTree, ProposedBlock, ProvenBlock,
    },
    note::Nullifier,
    transaction::ChainMmr,
    Digest, Word,
};

use crate::errors::ProvenBlockError;

// LOCAL BLOCK PROVER
// ================================================================================================

/// A local prover for blocks, proving a [`ProposedBlock`] and returning a [`ProvenBlock`].
pub struct LocalBlockProver {}

impl LocalBlockProver {
    /// Creates a new [`LocalBlockProver`] instance.
    pub fn new(_proof_security_level: u32) -> Self {
        // TODO: This will eventually take the security level as a parameter, but until we verify
        // batches it is ignored.
        Self {}
    }

    /// Proves the provided [`ProposedBlock`] into a [`ProvenBlock`].
    ///
    /// For now this does not actually verify the batches or create a block proof, but will be added
    /// in the future.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the account witnesses provided in the proposed block result in a different account tree
    ///   root than the contained previous block header commits to.
    /// - the nullifier witnesses provided in the proposed block result in a different nullifier
    ///   tree root than the contained previous block header commits to.
    pub fn prove(&self, proposed_block: ProposedBlock) -> Result<ProvenBlock, ProvenBlockError> {
        self.prove_without_verification_inner(proposed_block)
    }

    /// Proves the provided [`ProposedBlock`] into a [`ProvenBlock`], **without verifying batches
    /// and proving the block**.
    ///
    /// This is exposed for testing purposes.
    #[cfg(any(feature = "testing", test))]
    pub fn prove_without_verification(
        &self,
        proposed_block: ProposedBlock,
    ) -> Result<ProvenBlock, ProvenBlockError> {
        self.prove_without_verification_inner(proposed_block)
    }

    /// Proves the provided [`ProposedBlock`] into a [`ProvenBlock`].
    ///
    /// The assumptions of this method are that the checks made by construction of a
    /// [`ProposedBlock`] are enforced.
    ///
    /// See [`Self::prove`] for more details.
    fn prove_without_verification_inner(
        &self,
        proposed_block: ProposedBlock,
    ) -> Result<ProvenBlock, ProvenBlockError> {
        // Get the block number and timestamp of the new block and compute the tx commitment.
        // --------------------------------------------------------------------------------------------

        let block_num = proposed_block.block_num();
        let timestamp = proposed_block.timestamp();
        let tx_hash = BlockHeader::compute_tx_commitment(proposed_block.affected_accounts());

        // Split the proposed block into its parts.
        // --------------------------------------------------------------------------------------------

        // TODO: We should include the batches in the block, right? If so, should we transform them
        // into a `VerifiedBatch` struct, with the (eventually present) ZK proof removed and
        // the batch note trees updated after erased notes on the block level have been
        // taken into account?
        let (
            _batches,
            mut account_updated_witnesses,
            output_note_batches,
            block_note_tree,
            created_nullifiers,
            chain_mmr,
            prev_block_header,
        ) = proposed_block.into_parts();

        let prev_block_commitment = prev_block_header.hash();

        // Get the root of the block note tree.
        // --------------------------------------------------------------------------------------------

        // TODO: Do we need the full tree in proposed block or would the root be sufficient? We
        // can reconstruct the tree from the output note batches, so the question is whether it
        // should be included in the proven block to be readily accessible or if recomputing it is
        // fine.
        let note_root = block_note_tree.root();

        // Insert the created nullifiers into the nullifier tree to compute its new root.
        // --------------------------------------------------------------------------------------------

        let (created_nullifiers, new_nullifier_root) =
            compute_nullifiers(created_nullifiers, &prev_block_header, block_num)?;

        // Insert the previous block header into the block chain MMR to get the new chain root.
        // --------------------------------------------------------------------------------------------

        let new_chain_root = compute_chain_root(chain_mmr, prev_block_header);

        // Insert the state commitments of updated accounts into the account tree to compute its new
        // root.
        // --------------------------------------------------------------------------------------------

        let new_account_root = compute_account_root(&mut account_updated_witnesses)?;

        // Transform the account update witnesses into block account updates.
        // --------------------------------------------------------------------------------------------

        let updated_accounts = account_updated_witnesses
            .into_iter()
            .map(|(account_id, update_witness)| {
                let (
                    _initial_state_commitment,
                    final_state_commitment,
                    // Note that compute_account_root took out this value so it should not be used.
                    _initial_state_proof,
                    details,
                    transactions,
                ) = update_witness.into_parts();
                BlockAccountUpdate::new(account_id, final_state_commitment, details, transactions)
            })
            .collect();

        // Construct the new block header.
        // --------------------------------------------------------------------------------------------

        // TODO: Where is this defined? Should we rename this to `protocol_version`, if it is that?
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

        // Construct the new proven block.
        // --------------------------------------------------------------------------------------------

        let proven_block = ProvenBlock::new_unchecked(
            header,
            updated_accounts,
            output_note_batches,
            created_nullifiers,
        );

        Ok(proven_block)
    }
}

/// Computes the new nullifier root by inserting the nullifier witnesses into a partial nullifier
/// tree and marking each nullifier as spent in the given block number. Returns the list of
/// nullifiers and the new nullifier tree root.
fn compute_nullifiers(
    created_nullifiers: BTreeMap<Nullifier, NullifierWitness>,
    prev_block_header: &BlockHeader,
    block_num: BlockNumber,
) -> Result<(Vec<Nullifier>, Digest), ProvenBlockError> {
    let nullifiers: Vec<Nullifier> = created_nullifiers.keys().copied().collect();

    let mut partial_nullifier_tree = PartialNullifierTree::new();

    // First, reconstruct the current nullifier tree with the merkle paths of the nullifiers we want
    // to update.
    // Due to the guarantees of ProposedBlock we can safely assume that each nullifier is mapped to
    // its corresponding nullifier witness, so we don't have to check again whether they match.
    for witness in created_nullifiers.into_values() {
        partial_nullifier_tree
            .add_nullifier_witness(witness)
            .map_err(ProvenBlockError::NullifierWitnessRootMismatch)?;
    }

    debug_assert_eq!(
        partial_nullifier_tree.root(),
        prev_block_header.nullifier_root(),
        "partial nullifier tree root should match nullifier root of previous block header as validated in the loop"
    );

    // Second, mark each nullifier as spent in the tree. Note that checking whether each nullifier
    // is unspent is checked as part of the proposed block.
    for nullifier in nullifiers.iter().copied() {
        // SAFETY: As mentioned above, we can safely assume that each nullifier's witness was added
        // and every nullifier should be tracked by the partial tree and therefore updatable.
        partial_nullifier_tree.mark_spent(nullifier, block_num).expect(
            "nullifier's merkle path should have been added to the partial tree and the nullifier should be unspent",
        );
    }

    Ok((nullifiers, partial_nullifier_tree.root()))
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
/// updated to an Smt, we can use a PartialSmt instead.
fn compute_account_root(
    updated_accounts: &mut Vec<(AccountId, AccountUpdateWitness)>,
) -> Result<Digest, ProvenBlockError> {
    let mut partial_account_tree = PartialMerkleTree::new();

    // First reconstruct the current account tree from the provided merkle paths.
    for (account_id, witness) in updated_accounts.iter_mut() {
        let account_leaf_index = LeafIndex::from(*account_id);
        // Shouldn't the value in PartialMerkleTree::add_path be a Word instead of a Digest?
        // PartialMerkleTree::update_leaf (below) takes a Word as a value, so this seems
        // inconsistent.
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
        let account_leaf_index = LeafIndex::from(*account_id);
        partial_account_tree
            .update_leaf(account_leaf_index.value(), Word::from(witness.final_state_commitment()))
            .expect("every account leaf should have been inserted in the first loop");
    }

    Ok(partial_account_tree.root())
}
