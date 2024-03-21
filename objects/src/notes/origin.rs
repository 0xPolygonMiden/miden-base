use alloc::string::ToString;

use super::{
    ByteReader, ByteWriter, Deserializable, DeserializationError, Digest, NoteError, Serializable,
    NOTE_TREE_DEPTH,
};
use crate::crypto::merkle::{MerklePath, NodeIndex};

/// Contains information about the origin of a note.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteOrigin {
    pub block_num: u32,
    pub node_index: NodeIndex, // TODO: should be a u32 because the depth is always the same
}

/// Contains the data required to prove inclusion of a note in the canonical chain.
///
/// block_num  - the block number the note was created in.
/// sub_hash   - the sub hash of the block the note was created in.
/// note_root  - the note root of the block the note was created in.
/// note_index - the index of the note in the note Merkle tree of the block the note was created
///              in.
/// note_path  - the Merkle path to the note in the note Merkle tree of the block the note was
///              created in.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteInclusionProof {
    origin: NoteOrigin,
    sub_hash: Digest,
    note_root: Digest,
    note_path: MerklePath,
}

impl NoteInclusionProof {
    /// Creates a new note origin.
    pub fn new(
        block_num: u32,
        sub_hash: Digest,
        note_root: Digest,
        index: u64,
        note_path: MerklePath,
    ) -> Result<Self, NoteError> {
        let node_index = NodeIndex::new(NOTE_TREE_DEPTH, index)
            .map_err(|e| NoteError::invalid_origin_index(e.to_string()))?;
        Ok(Self {
            origin: NoteOrigin { block_num, node_index },
            sub_hash,
            note_root,
            note_path,
        })
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the sub hash of the block header the note was created in.
    pub fn sub_hash(&self) -> Digest {
        self.sub_hash
    }

    /// Returns the note root of the block header the note was created in.
    pub fn note_root(&self) -> Digest {
        self.note_root
    }

    /// Returns the origin of the note.
    pub fn origin(&self) -> &NoteOrigin {
        &self.origin
    }

    /// Returns the Merkle path to the note in the note Merkle tree of the block the note was
    /// created in.
    pub fn note_path(&self) -> &MerklePath {
        &self.note_path
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteOrigin {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_u32(self.block_num);
        self.node_index.write_into(target);
    }
}

impl Deserializable for NoteOrigin {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let block_num = source.read_u32()?;
        let node_index = NodeIndex::read_from(source)?;

        Ok(Self { block_num, node_index })
    }
}

impl Serializable for NoteInclusionProof {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.origin.write_into(target);
        self.sub_hash.write_into(target);
        self.note_root.write_into(target);
        self.note_path.write_into(target);
    }
}

impl Deserializable for NoteInclusionProof {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let origin = NoteOrigin::read_from(source)?;
        let sub_hash = Digest::read_from(source)?;
        let note_root = Digest::read_from(source)?;
        let note_path = MerklePath::read_from(source)?;

        Ok(Self { origin, sub_hash, note_root, note_path })
    }
}
