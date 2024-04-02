use miden_crypto::{
    hash::rpo::RpoDigest,
    merkle::{LeafIndex, MerkleError, MerklePath, SimpleSmt},
};

use crate::{
    notes::{NoteMetadata, NOTE_LEAF_DEPTH},
    BLOCK_OUTPUT_NOTES_TREE_DEPTH, MAX_NOTES_PER_BATCH,
};

/// Wrapper over [SimpleSmt] for notes tree
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct BlockNoteTree(SimpleSmt<BLOCK_OUTPUT_NOTES_TREE_DEPTH>);

impl BlockNoteTree {
    /// Returns a new [BlockOutputNotesTree].
    ///
    /// All leaves in the returned tree are set to [ZERO; 4].
    pub fn new() -> Self {
        Self(SimpleSmt::new().expect("Unreachable"))
    }

    /// Returns a new [BlockOutputNotesTree] instantiated with entries set as specified by the provided entries.
    ///
    /// Entry format: (batch_index, note_index, (note_id, note_metadata)).
    ///
    /// All leaves omitted from the entries list are set to [ZERO; 4].
    ///
    /// # Errors
    /// Returns an error if:
    /// - The number of entries exceeds the maximum notes tree capacity, that is 2^21.
    /// - The provided entries contain multiple values for the same key.
    pub fn with_entries(
        entries: impl IntoIterator<Item = (usize, usize, (RpoDigest, NoteMetadata))>,
    ) -> Result<Self, MerkleError> {
        let interleaved =
            entries.into_iter().flat_map(|(batch_index, note_index, (note_id, metadata))| {
                let id_index = Self::leaf_index(batch_index, note_index);
                [(id_index, note_id.into()), (id_index + 1, metadata.into())]
            });

        SimpleSmt::with_leaves(interleaved).map(Self)
    }

    /// Returns the root of the tree
    pub fn root(&self) -> RpoDigest {
        self.0.root()
    }

    /// Returns merkle path for the note with specified batch/note indexes
    pub fn merkle_path(
        &self,
        batch_idx: usize,
        note_idx_in_batch: usize,
    ) -> Result<MerklePath, MerkleError> {
        let leaf_index =
            LeafIndex::<NOTE_LEAF_DEPTH>::new(Self::note_index(batch_idx, note_idx_in_batch))?;

        Ok(self.0.open(&leaf_index).path)
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    fn note_index(batch_idx: usize, note_idx_in_batch: usize) -> u64 {
        (batch_idx * MAX_NOTES_PER_BATCH + note_idx_in_batch) as u64
    }

    fn leaf_index(batch_idx: usize, note_idx_in_batch: usize) -> u64 {
        Self::note_index(batch_idx, note_idx_in_batch) * 2
    }
}

impl Default for BlockNoteTree {
    fn default() -> Self {
        Self::new()
    }
}
