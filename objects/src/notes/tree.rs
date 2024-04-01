use miden_crypto::{
    hash::rpo::RpoDigest,
    merkle::{LeafIndex, MerkleError, SimpleSmt},
};

use super::NoteMetadata;
use crate::{BATCH_OUTPUT_NOTES_TREE_DEPTH, BLOCK_OUTPUT_NOTES_TREE_DEPTH, MAX_NOTES_PER_BATCH};

/// Wrapper over [SimpleSmt] for notes tree
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct NotesTree<const DEPTH: u8>(SimpleSmt<DEPTH>);

impl<const DEPTH: u8> NotesTree<DEPTH> {
    /// Returns a new [NotesTree].
    ///
    /// All leaves in the returned tree are set to [ZERO; 4].
    ///
    /// # Errors
    /// Returns an error if DEPTH is 0 or is greater than 64.
    pub fn new() -> Result<Self, MerkleError> {
        SimpleSmt::new().map(Self)
    }

    /// Returns a new [NotesTree] instantiated with entries set as specified by the provided entries.
    ///
    /// Entry format: (note_index, (note_id, &note_metadata)).
    ///
    /// All leaves omitted from the entries list are set to [ZERO; 4].
    ///
    /// # Errors
    /// Returns an error if:
    /// - The depth is 0 or is greater than 64.
    /// - The number of entries exceeds the maximum notes tree capacity, that is 2^21.
    /// - The provided entries contain multiple values for the same key.
    pub fn with_entries<'a>(
        entries: impl IntoIterator<Item = (usize, (RpoDigest, &'a NoteMetadata))>,
    ) -> Result<Self, MerkleError> {
        let entries = entries.into_iter().flat_map(|(note_index, (id, metadata))| {
            let id_index = note_index as u64 * 2;
            [(id_index, id.into()), (id_index + 1, metadata.into())]
        });

        SimpleSmt::with_leaves(entries).map(Self)
    }

    /// Returns the root of the tree
    pub fn root(&self) -> RpoDigest {
        self.0.root()
    }

    /// Inserts an entry at the specified note index.
    /// Recall that by definition, any key that hasn't been updated is associated with [`EMPTY_WORD`].
    ///
    /// This also recomputes all hashes between the leaf (associated with the key) and the root,
    /// updating the root itself.
    pub fn insert(
        &mut self,
        note_index: usize,
        note_id: RpoDigest,
        metadata: &NoteMetadata,
    ) -> Result<(), MerkleError> {
        let id_index = note_index as u64 * 2;
        self.0.insert(LeafIndex::<DEPTH>::new(id_index)?, note_id.into());
        self.0.insert(LeafIndex::<DEPTH>::new(id_index + 1)?, metadata.into());

        Ok(())
    }
}

pub type BatchOutputNotesTree = NotesTree<BATCH_OUTPUT_NOTES_TREE_DEPTH>;

/// Wrapper over [SimpleSmt] for notes tree
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct BlockOutputNotesTree(NotesTree<BLOCK_OUTPUT_NOTES_TREE_DEPTH>);

impl BlockOutputNotesTree {
    /// Returns a new [BlockOutputNotesTree].
    ///
    /// All leaves in the returned tree are set to [ZERO; 4].
    pub fn new() -> Self {
        Self(NotesTree::new().expect("Unreachable"))
    }

    /// Returns a new [BlockOutputNotesTree] instantiated with entries set as specified by the provided entries.
    ///
    /// Entry format: (batch_index, note_index, (note_id, &note_metadata)).
    ///
    /// All leaves omitted from the entries list are set to [ZERO; 4].
    ///
    /// # Errors
    /// Returns an error if:
    /// - The number of entries exceeds the maximum notes tree capacity, that is 2^21.
    /// - The provided entries contain multiple values for the same key.
    pub fn with_entries<'a>(
        entries: impl IntoIterator<Item = (usize, usize, (RpoDigest, &'a NoteMetadata))>,
    ) -> Result<Self, MerkleError> {
        let entries = entries.into_iter().map(|(batch_index, note_index, note)| {
            (Self::note_index(batch_index, note_index), note)
        });

        NotesTree::with_entries(entries).map(Self)
    }

    /// Returns the root of the tree
    pub fn root(&self) -> RpoDigest {
        self.0.root()
    }

    /// Inserts an entry at the specified batch/note index.
    /// Recall that by definition, any key that hasn't been updated is associated with [`EMPTY_WORD`].
    ///
    /// This also recomputes all hashes between the leaf (associated with the key) and the root,
    /// updating the root itself.
    pub fn insert(
        &mut self,
        batch_index: usize,
        note_index: usize,
        note_id: RpoDigest,
        metadata: &NoteMetadata,
    ) -> Result<(), MerkleError> {
        self.0.insert(Self::note_index(batch_index, note_index), note_id, metadata)
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    const fn note_index(batch_index: usize, note_index: usize) -> usize {
        batch_index * MAX_NOTES_PER_BATCH / 2 + note_index
    }
}

impl Default for BlockOutputNotesTree {
    fn default() -> Self {
        Self::new()
    }
}
