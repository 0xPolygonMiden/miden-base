use miden_crypto::merkle::{PartialSmt, SmtProof};
use vm_core::{Word, EMPTY_WORD};
use vm_processor::Digest;

use crate::{
    block::{nullifier_tree::block_num_to_leaf_value, BlockNumber},
    note::Nullifier,
};

/// TODO
pub struct PartialNullifierTree(PartialSmt);

impl PartialNullifierTree {
    pub const UNSPENT_NULLIFIER_VALUE: Word = EMPTY_WORD;

    pub fn new() -> Self {
        PartialNullifierTree(PartialSmt::new())
    }

    pub fn add_nullifier_path(&mut self, nullifier: Nullifier, proof: SmtProof) {
        let (path, leaf) = proof.into_parts();

        let current_nullifier_value = leaf
            .entries()
            .iter()
            .find_map(|(key, value)| (*key == nullifier.inner()).then_some(value))
            .expect("TODO: error");

        if *current_nullifier_value != Self::UNSPENT_NULLIFIER_VALUE {
            todo!("error: nullifier is already spent")
        }

        self.0.add_path(leaf, path).expect("TODO: error");
    }

    pub fn mark_spent(&mut self, nullifier: Nullifier, block_num: BlockNumber) {
        self.0
            .insert(nullifier.inner(), block_num_to_leaf_value(block_num))
            .expect("TODO: error");
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
