use miden_crypto::merkle::{PartialSmt, SmtProof};
use vm_core::{Word, EMPTY_WORD};
use vm_processor::Digest;

use crate::{
    block::{nullifier_tree::block_num_to_leaf_value, BlockNumber},
    note::Nullifier,
};

pub struct PartialNullifierTree(PartialSmt);

impl PartialNullifierTree {
    pub const UNSPENT_NULLIFIER_VALUE: Word = EMPTY_WORD;

    pub fn new() -> Self {
        PartialNullifierTree(PartialSmt::new())
    }

    pub fn add_nullifier_path(&mut self, nullifier: Nullifier, proof: SmtProof) {
        let (path, leaf) = proof.into_parts();

        for (key, value) in leaf.into_entries() {
            // We only need to check that the nullifier is unspent, the other key-value pairs of the
            // leaf entries are unimportant here but still need to be added to the SMT to produce
            // the correct nullifier tree root.
            if key == nullifier.inner() && value != Self::UNSPENT_NULLIFIER_VALUE {
                todo!("error: nullifier is already spent")
            }

            self.0.add_path(key, value, path.clone()).expect("TODO: Error");
        }
    }

    pub fn mark_spent(&mut self, nullifier: Nullifier, block_num: BlockNumber) {
        self.0.insert(nullifier.inner(), block_num_to_leaf_value(block_num));
    }

    pub fn root(&self) -> Digest {
        self.0.root()
    }
}
