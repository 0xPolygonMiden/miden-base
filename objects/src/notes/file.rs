use vm_core::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::DeserializationError;

use super::{Note, NoteDetails, NoteInclusionProof};

// NOTE FILE
// ================================================================================================

/// A serialized representation of a note.
pub enum NoteFile {
    /// The note has not yet been recorded on chain.
    Details(NoteDetails),
    /// The note has been recorded on chain.
    Recorded(Note, NoteInclusionProof),
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteFile {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            NoteFile::Details(details) => {
                target.write_u8(0);
                details.write_into(target);
            },
            NoteFile::Recorded(note, proof) => {
                target.write_u8(1);
                note.write_into(target);
                proof.write_into(target);
            },
        }
    }
}

impl Deserializable for NoteFile {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        match source.read_u8()? {
            0 => Ok(NoteFile::Details(NoteDetails::read_from(source)?)),
            1 => {
                let note = Note::read_from(source)?;
                let proof = NoteInclusionProof::read_from(source)?;
                Ok(NoteFile::Recorded(note, proof))
            },
            v => {
                Err(DeserializationError::InvalidValue(format!("Unknown variant {v} for NoteFile")))
            },
        }
    }
}
