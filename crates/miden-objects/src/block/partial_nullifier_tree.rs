use miden_crypto::merkle::{MerkleError, PartialSmt};
use vm_core::{Felt, FieldElement, Word, EMPTY_WORD};
use vm_processor::Digest;

use crate::{
    block::{BlockNumber, NullifierWitness},
    note::Nullifier,
};

/// The partial sparse merkle tree containing the nullifiers of consumed notes. This is the partial
/// variant of [`NullifierTree`].
pub struct PartialNullifierTree(PartialSmt);

impl PartialNullifierTree {
    /// The leaf value of an unspent nullifier.
    pub const UNSPENT_NULLIFIER: Word = EMPTY_WORD;

    pub fn new() -> Self {
        PartialNullifierTree(PartialSmt::new())
    }

    // TODO: Document errors.
    pub fn add_nullifier_witness(&mut self, witness: NullifierWitness) -> Result<(), MerkleError> {
        let (path, leaf) = witness.into_proof().into_parts();
        self.0.add_path(leaf, path)
    }

    // TODO: Document errors.
    pub fn mark_spent(
        &mut self,
        nullifier: Nullifier,
        block_num: BlockNumber,
    ) -> Result<(), MerkleError> {
        self.0.insert(nullifier.inner(), block_num_to_leaf_value(block_num)).map(|_| ())
    }

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
pub(super) fn block_num_to_leaf_value(block: BlockNumber) -> Word {
    [Felt::from(block), Felt::ZERO, Felt::ZERO, Felt::ZERO]
}
