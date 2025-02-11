use alloc::vec::Vec;

use miden_crypto::{
    hash::rpo::RpoDigest,
    merkle::{MerkleError, SimpleSmt},
};
use vm_core::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::DeserializationError;

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
}

// SERIALIZATION
// ================================================================================================

impl Serializable for BatchNoteTree {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.0.leaves().collect::<Vec<_>>().write_into(target);
    }
}

impl Deserializable for BatchNoteTree {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let leaves = Vec::read_from(source)?;
        let smt = SimpleSmt::with_contiguous_leaves(leaves.into_iter()).map_err(|err| {
            DeserializationError::UnknownError(format!(
                "failed to deserialize BatchNoteTree: {err}"
            ))
        })?;
        Ok(Self(smt))
    }
}
