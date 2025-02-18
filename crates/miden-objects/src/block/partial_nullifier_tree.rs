use miden_crypto::merkle::PartialSmt;
use vm_core::{Felt, FieldElement, Word, EMPTY_WORD};
use vm_processor::Digest;

use crate::{
    block::{BlockNumber, NullifierWitness},
    errors::NullifierTreeError,
    note::Nullifier,
};

/// The partial sparse merkle tree containing the nullifiers of consumed notes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialNullifierTree(PartialSmt);

impl PartialNullifierTree {
    /// The leaf value of an unspent nullifier.
    pub const UNSPENT_NULLIFIER: Word = EMPTY_WORD;

    /// Creates a new, empty partial nullifier tree.
    pub fn new() -> Self {
        PartialNullifierTree(PartialSmt::new())
    }

    /// Adds the given nullifier witness to the partial tree and tracks it. Once a nullifier has
    /// been added to the tree, it can be marked as spent using [`Self::mark_spent`].
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - after the witness' merkle path was added the partial nullifier tree has a different root
    ///   than before it was added.
    pub fn add_nullifier_witness(
        &mut self,
        witness: NullifierWitness,
    ) -> Result<(), NullifierTreeError> {
        let (path, leaf) = witness.into_proof().into_parts();
        self.0.add_path(leaf, path).map_err(NullifierTreeError::TreeRootConflict)
    }

    /// Marks the given nullifiers as spent at the given block number.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - a nullifier was already spent.
    /// - a nullifier is not tracked by this partial nullifier tree, that is, its
    ///   [`NullifierWitness`] was not added to the tree previously.
    pub fn mark_spent(
        &mut self,
        nullifiers: impl Iterator<Item = Nullifier>,
        block_num: BlockNumber,
    ) -> Result<(), NullifierTreeError> {
        for nullifier in nullifiers {
            self.mark_spent_single(nullifier, block_num)?;
        }

        Ok(())
    }

    /// Returns the root of the tree.
    pub fn root(&self) -> Digest {
        self.0.root()
    }

    /// Marks the given nullifier as spent at the given block number.
    ///
    /// # Errors
    ///
    /// See [`Self::mark_spent`] for the possible error conditions.
    fn mark_spent_single(
        &mut self,
        nullifier: Nullifier,
        block_num: BlockNumber,
    ) -> Result<(), NullifierTreeError> {
        let prev_nullifier_value = self
            .0
            .insert(nullifier.inner(), block_num_to_leaf_value(block_num))
            .map_err(|source| NullifierTreeError::UntrackedNullifier { nullifier, source })?;

        if prev_nullifier_value != Self::UNSPENT_NULLIFIER {
            Err(NullifierTreeError::NullifierAlreadySpent(nullifier))
        } else {
            Ok(())
        }
    }
}

impl Default for PartialNullifierTree {
    fn default() -> Self {
        Self::new()
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Returns the nullifier's leaf value in the SMT by its block number.
fn block_num_to_leaf_value(block: BlockNumber) -> Word {
    [Felt::from(block), Felt::ZERO, Felt::ZERO, Felt::ZERO]
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use miden_crypto::merkle::Smt;
    use winter_rand_utils::rand_array;

    use super::*;

    /// Test that using a stale nullifier witness together with a current one results in a different
    /// tree root and thus an error.
    #[test]
    fn partial_nullifier_tree_root_mismatch() {
        let key0 = Digest::from(Word::from(rand_array()));
        let key1 = Digest::from(Word::from(rand_array()));
        let key2 = Digest::from(Word::from(rand_array()));

        let value0 = EMPTY_WORD;
        let value1 = Word::from(rand_array());
        let value2 = EMPTY_WORD;

        let kv_pairs = vec![(key0, value0)];

        let mut full = Smt::with_entries(kv_pairs).unwrap();
        let stale_proof0 = full.open(&key0);
        // Insert a non-empty value so the nullifier tree's root changes.
        full.insert(key1, value1);
        full.insert(key2, value2);
        let proof2 = full.open(&key2);

        assert_ne!(stale_proof0.compute_root(), proof2.compute_root());

        let mut partial = PartialNullifierTree::new();

        partial.add_nullifier_witness(NullifierWitness::new(stale_proof0)).unwrap();
        let error = partial.add_nullifier_witness(NullifierWitness::new(proof2)).unwrap_err();

        assert_matches!(error, NullifierTreeError::TreeRootConflict(_));
    }
}
