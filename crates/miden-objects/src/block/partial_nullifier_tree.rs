use miden_crypto::merkle::{MerkleError, PartialSmt};
use vm_core::{Felt, FieldElement, Word, EMPTY_WORD};
use vm_processor::Digest;

use crate::{
    block::{BlockNumber, NullifierWitness},
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
    pub fn add_nullifier_witness(&mut self, witness: NullifierWitness) -> Result<(), MerkleError> {
        let (path, leaf) = witness.into_proof().into_parts();
        self.0.add_path(leaf, path)
    }

    /// Marks the given nullifier as spent at the given block number.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the nullifier is not tracked by this partial nullifier tree, that is, its
    ///   [`NullifierWitness`] was not added to the tree previously.
    pub fn mark_spent(
        &mut self,
        nullifier: Nullifier,
        block_num: BlockNumber,
    ) -> Result<(), MerkleError> {
        self.0.insert(nullifier.inner(), block_num_to_leaf_value(block_num)).map(|_| ())
    }

    /// Returns the root of the tree.
    pub fn root(&self) -> Digest {
        self.0.root()
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
