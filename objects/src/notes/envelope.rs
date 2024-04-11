use alloc::vec::Vec;

use super::{
    ByteReader, ByteWriter, Deserializable, DeserializationError, Felt, Note, NoteId, NoteMetadata,
    NoteType, Serializable, Word,
};
use crate::NoteError;

// NOTE ENVELOPE
// ================================================================================================

/// Holds the strictly required, public information of a note.
///
/// See [NoteId] and [NoteMetadata] for additional details.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteEnvelope {
    note_id: NoteId,
    note_metadata: NoteMetadata,
}

impl NoteEnvelope {
    /// Returns a new [NoteEnvelope] object.
    pub fn new(note_id: NoteId, note_metadata: NoteMetadata) -> Result<Self, NoteError> {
        let note_type = note_metadata.note_type();
        if note_type != NoteType::OffChain {
            return Err(NoteError::InvalidNoteType(note_type));
        }
        Ok(Self { note_id, note_metadata })
    }

    /// Returns the note's identifier.
    ///
    /// The [NoteId] value is both an unique identifier and a commitment to the note.
    pub fn id(&self) -> NoteId {
        self.note_id
    }

    /// Returns the note's metadata.
    pub fn metadata(&self) -> &NoteMetadata {
        &self.note_metadata
    }
}

impl From<NoteEnvelope> for [Felt; 8] {
    fn from(note_envelope: NoteEnvelope) -> Self {
        (&note_envelope).into()
    }
}

impl From<NoteEnvelope> for [Word; 2] {
    fn from(note_envelope: NoteEnvelope) -> Self {
        (&note_envelope).into()
    }
}

impl From<NoteEnvelope> for [u8; 64] {
    fn from(note_envelope: NoteEnvelope) -> Self {
        (&note_envelope).into()
    }
}

impl From<&NoteEnvelope> for [Felt; 8] {
    fn from(note_envelope: &NoteEnvelope) -> Self {
        let mut elements: [Felt; 8] = Default::default();
        elements[..4].copy_from_slice(note_envelope.note_id.as_elements());
        elements[4..].copy_from_slice(&Word::from(note_envelope.metadata()));
        elements
    }
}

impl From<&NoteEnvelope> for [Word; 2] {
    fn from(note_envelope: &NoteEnvelope) -> Self {
        let mut elements: [Word; 2] = Default::default();
        elements[0].copy_from_slice(note_envelope.note_id.as_elements());
        elements[1].copy_from_slice(&Word::from(note_envelope.metadata()));
        elements
    }
}

impl From<&NoteEnvelope> for [u8; 64] {
    fn from(note_envelope: &NoteEnvelope) -> Self {
        let mut elements: [u8; 64] = [0; 64];
        let note_metadata_bytes = Word::from(note_envelope.metadata())
            .iter()
            .flat_map(|x| x.as_int().to_le_bytes())
            .collect::<Vec<u8>>();
        elements[..32].copy_from_slice(&note_envelope.note_id.as_bytes());
        elements[32..].copy_from_slice(&note_metadata_bytes);
        elements
    }
}

impl From<Note> for NoteEnvelope {
    fn from(note: Note) -> Self {
        (&note).into()
    }
}

impl From<&Note> for NoteEnvelope {
    fn from(note: &Note) -> Self {
        Self {
            note_id: note.id(),
            note_metadata: *note.metadata(),
        }
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteEnvelope {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.note_id.write_into(target);
        self.note_metadata.write_into(target);
    }
}

impl Deserializable for NoteEnvelope {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let note_id = NoteId::read_from(source)?;
        let note_metadata = NoteMetadata::read_from(source)?;

        Ok(Self { note_id, note_metadata })
    }
}
