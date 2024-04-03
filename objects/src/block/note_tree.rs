use alloc::string::ToString;

use miden_crypto::{
    hash::rpo::RpoDigest,
    merkle::{LeafIndex, MerkleError, MerklePath, SimpleSmt},
};

use crate::{
    notes::{NoteMetadata, NOTE_LEAF_DEPTH},
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    BLOCK_OUTPUT_NOTES_TREE_DEPTH, MAX_NOTES_PER_BATCH,
};

/// Wrapper over [SimpleSmt<BLOCK_OUTPUT_NOTES_TREE_DEPTH>] for notes tree.
///
/// Each note is stored as two adjacent leaves: odd leaf for id, even leaf for metadata hash.
/// Id leaf index is calculated as [(batch_idx * MAX_NOTES_PER_BATCH + note_idx_in_batch) * 2].
/// Metadata hash leaf is stored the next after id leaf: [id_index + 1].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct BlockNoteTree(SimpleSmt<BLOCK_OUTPUT_NOTES_TREE_DEPTH>);

impl BlockNoteTree {
    /// Returns a new [BlockNoteTree] instantiated with entries set as specified by the provided entries.
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
        Self(SimpleSmt::new().expect("Unreachable"))
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
