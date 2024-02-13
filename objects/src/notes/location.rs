use super::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable};

// NOTE LOCATION
// ================================================================================================

/// Location at which the note is recorded in the chain.
///
/// The location consists of two elements:
/// - The number of the block at which the note was recorded in the chain.
/// - The index of the note in the block's note tree.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteLocation {
    block_num: u32,
    note_index: u32, // TODO: change to u16
}

impl NoteLocation {
    pub fn new(block_num: u32, note_index: u32) -> Self {
        Self { block_num, note_index }
    }

    /// Returns the number of the block at which the note was recorded in the chain.
    pub fn block_num(&self) -> u32 {
        self.block_num
    }

    /// Return the index of thn note in the block's note tree.
    pub fn note_index(&self) -> u32 {
        self.note_index
    }
}

impl Serializable for NoteLocation {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_u32(self.block_num);
        self.note_index.write_into(target);
    }
}

impl Deserializable for NoteLocation {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let block_num = source.read_u32()?;
        let note_index = source.read_u32()?;
        Ok(Self { block_num, note_index })
    }
}
