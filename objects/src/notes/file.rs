use vm_core::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::DeserializationError;

use super::{Note, NoteDetails, NoteId, NoteInclusionProof};

// NOTE FILE
// ================================================================================================

/// A serialized representation of a note.
pub enum NoteFile {
    /// The note's details aren't known.
    NoteId(NoteId),
    /// The note has not yet been recorded on chain.
    NoteDetails(NoteDetails),
    /// The note has been recorded on chain.
    NoteWithProof(Note, NoteInclusionProof),
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteFile {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_bytes("note".as_bytes());
        match self {
            NoteFile::NoteId(note_id) => {
                target.write_u8(0);
                note_id.write_into(target);
            },
            NoteFile::NoteDetails(details) => {
                target.write_u8(1);
                details.write_into(target);
            },
            NoteFile::NoteWithProof(note, proof) => {
                target.write_u8(2);
                note.write_into(target);
                proof.write_into(target);
            },
        }
    }
}

impl Deserializable for NoteFile {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let magic_value = source.read_string(4)?;
        if magic_value != "note" {
            return Err(DeserializationError::InvalidValue(format!(
                "Invalid note file marker: {magic_value}"
            )));
        }
        match source.read_u8()? {
            0 => Ok(NoteFile::NoteId(NoteId::read_from(source)?)),
            1 => Ok(NoteFile::NoteDetails(NoteDetails::read_from(source)?)),
            2 => {
                let note = Note::read_from(source)?;
                let proof = NoteInclusionProof::read_from(source)?;
                Ok(NoteFile::NoteWithProof(note, proof))
            },
            v => {
                Err(DeserializationError::InvalidValue(format!("Unknown variant {v} for NoteFile")))
            },
        }
    }
}
