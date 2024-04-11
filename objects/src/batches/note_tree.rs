use miden_crypto::{
    hash::rpo::RpoDigest,
    merkle::{MerkleError, SimpleSmt},
};

use crate::{
    notes::{NoteId, NoteMetadata},
    BATCH_OUTPUT_NOTES_TREE_DEPTH,
};

/// Wrapper over [SimpleSmt<BATCH_OUTPUT_NOTES_TREE_DEPTH>] for batch note tree.
///
/// Each note is stored as two adjacent leaves: odd leaf for id, even leaf for metadata hash.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct BatchNoteTree(SimpleSmt<BATCH_OUTPUT_NOTES_TREE_DEPTH>);

impl BatchNoteTree {
    /// Wrapper around [`SimpleSmt::with_contiguous_leaves`] which populates notes at contiguous indices
    /// starting at index 0.
    ///
    /// # Errors
    /// Returns an error if the number of entries exceeds the maximum tree capacity, that is 2^{depth}.
    pub fn with_contiguous_leaves<'a>(
        entries: impl IntoIterator<Item = (NoteId, &'a NoteMetadata)>,
    ) -> Result<Self, MerkleError> {
        let interleaved = entries
            .into_iter()
            .flat_map(|(note_id, metadata)| [note_id.into(), metadata.into()]);

        SimpleSmt::with_contiguous_leaves(interleaved).map(Self)
    }

    /// Returns the root of the tree
    pub fn root(&self) -> RpoDigest {
        self.0.root()
    }
}
