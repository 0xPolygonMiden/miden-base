use alloc::string::ToString;
use core::num::TryFromIntError;

use super::{
    ByteReader, ByteWriter, Deserializable, DeserializationError, NoteError, Serializable,
};
use crate::{crypto::merkle::MerklePath, MAX_BATCHES_PER_BLOCK, MAX_NOTES_PER_BATCH};

/// Contains information about the location of a note.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteLocation {
    /// The block number the note was created in.
    block_num: u32,

    /// The index of the note in the note Merkle tree of the block the note was created in.
    node_index_in_block: u32,
}

impl NoteLocation {
    /// Returns the block number the note was created in.
    pub fn block_num(&self) -> u32 {
        self.block_num
    }

    /// Returns the index of the note in the note Merkle tree of the block the note was created in.
    pub fn node_index(&self) -> u64 {
        self.node_index_in_block as u64
    }
}

/// Contains the data required to prove inclusion of a note in the canonical chain.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteInclusionProof {
    /// Details about the note's location.
    location: NoteLocation,

    /// The note's authentication Merkle path its block's the note root.
    note_path: MerklePath,
}

impl NoteInclusionProof {
    /// Returns a new [NoteInclusionProof].
    pub fn new(block_num: u32, index: u64, note_path: MerklePath) -> Result<Self, NoteError> {
        const HIGHEST_INDEX: usize = MAX_BATCHES_PER_BLOCK * MAX_NOTES_PER_BATCH - 1;
        let node_index_in_block = index
            .try_into()
            .map_err(|err: TryFromIntError| NoteError::InvalidLocationIndex(err.to_string()))?;
        if node_index_in_block as usize > HIGHEST_INDEX {
            return Err(NoteError::InvalidLocationIndex(format!(
                "Node index ({index}) is out of bounds (0..={HIGHEST_INDEX})."
            )));
        }
        let location = NoteLocation { block_num, node_index_in_block };

        Ok(Self { location, note_path })
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the location of the note.
    pub fn location(&self) -> &NoteLocation {
        &self.location
    }

    /// Returns the Merkle path to the note in the note Merkle tree of the block the note was
    /// created in.
    pub fn note_path(&self) -> &MerklePath {
        &self.note_path
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteLocation {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_u32(self.block_num);
        target.write_u32(self.node_index_in_block);
    }
}

impl Deserializable for NoteLocation {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let block_num = source.read_u32()?;
        let node_index_in_block = source.read_u32()?;

        Ok(Self { block_num, node_index_in_block })
    }
}

impl Serializable for NoteInclusionProof {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.location.write_into(target);
        self.note_path.write_into(target);
    }
}

impl Deserializable for NoteInclusionProof {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let location = NoteLocation::read_from(source)?;
        let note_path = MerklePath::read_from(source)?;

        Ok(Self { location, note_path })
    }
}
