use alloc::string::ToString;

use miden_crypto::{
    hash::rpo::RpoDigest,
    merkle::{LeafIndex, MerkleError, MerklePath, SimpleSmt},
};

use crate::{
    notes::NoteMetadata,
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    BLOCK_OUTPUT_NOTES_TREE_DEPTH, MAX_NOTES_PER_BATCH,
};

/// Wrapper over [SimpleSmt<BLOCK_OUTPUT_NOTES_TREE_DEPTH>] for notes tree.
///
/// Each note is stored as two adjacent leaves: odd leaf for id, even leaf for metadata hash.
/// ID's leaf index is calculated as [(batch_idx * MAX_NOTES_PER_BATCH + note_idx_in_batch) * 2].
/// Metadata hash leaf is stored the next after id leaf: [id_index + 1].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct BlockNoteTree(SimpleSmt<BLOCK_OUTPUT_NOTES_TREE_DEPTH>);

impl BlockNoteTree {
    /// Returns a new [BlockNoteTree] instantiated with entries set as specified by the provided entries.
    ///
    /// Entry format: (note_index, note_id, note_metadata).
    ///
    /// All leaves omitted from the entries list are set to [ZERO; 4].
    ///
    /// # Errors
    /// Returns an error if:
    /// - The number of entries exceeds the maximum notes tree capacity, that is 2^21.
    /// - The provided entries contain multiple values for the same key.
    pub fn with_entries(
        entries: impl IntoIterator<Item = (BlockNoteIndex, RpoDigest, NoteMetadata)>,
    ) -> Result<Self, MerkleError> {
        let interleaved = entries.into_iter().flat_map(|(index, note_id, metadata)| {
            let id_index = index.leaf_index();
            [(id_index, note_id.into()), (id_index + 1, metadata.into())]
        });

        SimpleSmt::with_leaves(interleaved).map(Self)
    }

    /// Returns the root of the tree
    pub fn root(&self) -> RpoDigest {
        self.0.root()
    }

    /// Returns merkle path for the note with specified batch/note indexes.
    ///
    /// The returned path is to the node which is the parent of both note and note metadata node.
    pub fn get_note_path(&self, index: BlockNoteIndex) -> Result<MerklePath, MerkleError> {
        // get the path to the leaf containing the note (path len = 21)
        let leaf_index = LeafIndex::new(index.leaf_index())?;

        // move up the path by removing the first node, this path now points to the parent of the
        // note path
        let note_path = self.0.open(&leaf_index).path[1..].to_vec();

        Ok(note_path.into())
    }
}

impl Default for BlockNoteTree {
    fn default() -> Self {
        Self(SimpleSmt::new().expect("Unreachable"))
    }
}

/// Index of a block note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockNoteIndex {
    batch_idx: usize,
    note_idx_in_batch: usize,
}

impl BlockNoteIndex {
    /// Creates a new [BlockNoteIndex].
    pub fn new(batch_idx: usize, note_idx_in_batch: usize) -> Self {
        Self { batch_idx, note_idx_in_batch }
    }

    /// Returns the batch index.
    pub fn batch_idx(&self) -> usize {
        self.batch_idx
    }

    /// Returns the note index in the batch.
    pub fn note_idx_in_batch(&self) -> usize {
        self.note_idx_in_batch
    }

    /// Returns an index to the node which the parent of both the note and note metadata.
    pub fn to_absolute_index(&self) -> u64 {
        (self.batch_idx() * MAX_NOTES_PER_BATCH + self.note_idx_in_batch()) as u64
    }

    fn leaf_index(&self) -> u64 {
        self.to_absolute_index() * 2
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for BlockNoteTree {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_u32(self.0.num_leaves() as u32);
        target.write_many(self.0.leaves());
    }
}

impl Deserializable for BlockNoteTree {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let count = source.read_u32()?;
        let leaves = source.read_many(count as usize)?;

        SimpleSmt::with_leaves(leaves)
            .map(Self)
            .map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use miden_crypto::{
        merkle::SimpleSmt,
        utils::{Deserializable, Serializable},
        Felt, ONE, ZERO,
    };

    use super::BlockNoteTree;

    #[test]
    fn test_serialization() {
        let data = core::iter::repeat(())
            .enumerate()
            .map(|(idx, ())| (idx as u64, [ONE, ZERO, ONE, Felt::new(idx as u64)]))
            .take(100);
        let initial_tree = BlockNoteTree(SimpleSmt::with_leaves(data).unwrap());

        let serialized = initial_tree.to_bytes();
        let deserialized_tree = BlockNoteTree::read_from_bytes(&serialized).unwrap();

        assert_eq!(deserialized_tree, initial_tree);
    }
}
