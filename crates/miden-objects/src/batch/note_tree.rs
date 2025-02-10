use miden_crypto::{
    hash::rpo::RpoDigest,
    merkle::{LeafIndex, MerkleError, SimpleSmt},
};
use vm_core::EMPTY_WORD;

use crate::{
    note::{compute_note_hash, NoteId, NoteMetadata},
    BATCH_NOTE_TREE_DEPTH,
};

/// Wrapper over [SimpleSmt<BATCH_NOTE_TREE_DEPTH>] for batch note tree.
///
/// Value of each leaf is computed as: `hash(note_id || note_metadata)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchNoteTree(SimpleSmt<BATCH_NOTE_TREE_DEPTH>);

impl BatchNoteTree {
    /// Wrapper around [`SimpleSmt::with_contiguous_leaves`] which populates notes at contiguous
    /// indices starting at index 0.
    ///
    /// # Errors
    /// Returns an error if the number of entries exceeds the maximum tree capacity, that is
    /// 2^{depth}.
    pub fn with_contiguous_leaves<'a>(
        entries: impl IntoIterator<Item = (NoteId, &'a NoteMetadata)>,
    ) -> Result<Self, MerkleError> {
        let leaves = entries
            .into_iter()
            .map(|(note_id, metadata)| compute_note_hash(note_id, metadata).into());

        SimpleSmt::with_contiguous_leaves(leaves).map(Self)
    }

    /// Returns the root of the tree
    pub fn root(&self) -> RpoDigest {
        self.0.root()
    }

    /// Returns the number of non-empty leaves in this tree.
    pub fn num_leaves(&self) -> usize {
        self.0.num_leaves()
    }

    /// Removes the entry at the given index form the tree by inserting [`EMPTY_WORD`].
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the `index` is equal to or exceeds
    ///   [`MAX_OUTPUT_NOTES_PER_BATCH`](crate::constants::MAX_OUTPUT_NOTES_PER_BATCH).
    pub fn remove(&mut self, index: u64) -> Result<(), MerkleError> {
        let key = LeafIndex::new(index)?;
        self.0.insert(key, EMPTY_WORD);

        Ok(())
    }

    pub fn into_smt(self) -> SimpleSmt<BATCH_NOTE_TREE_DEPTH> {
        self.0
    }
}
