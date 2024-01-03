use super::{Digest, Felt, Hasher, Note, Word};
use crate::utils::serde::{
    ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable,
};

// NOTE ID
// ================================================================================================

/// Returns a unique identifier of a note, which is simultaneously a commitment to the note.
///
/// Note ID is computed as:
///
/// hash(hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash), vault_hash).
///
/// This achieves the following properties:
/// - Every note can be reduced to a single unique ID.
/// - To compute a note ID, we do not need to know the note's serial_num. Knowing the hash
///   of the serial_num (as well as script hash, input hash, and note vault) is sufficient.
/// - Moreover, we define `recipient` as:
///     `hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash)`
///   This allows computing note ID from recipient and note vault.
/// - We compute hash of serial_num as hash(serial_num, [0; 4]) to simplify processing within
///   the VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NoteId(Digest);

impl NoteId {
    /// Returns a new [NoteId] instantiated from the provided note components.
    pub fn new(recipient: Digest, vault_commitment: Digest) -> Self {
        Self(Hasher::merge(&[recipient, vault_commitment]))
    }

    /// Returns the elements representation of this note ID.
    pub fn as_elements(&self) -> &[Felt] {
        self.0.as_elements()
    }

    /// Returns the byte representation of this note ID.
    pub fn as_bytes(&self) -> [u8; 32] {
        self.0.as_bytes()
    }

    /// Returns the digest defining this note ID.
    pub fn inner(&self) -> Digest {
        self.0
    }
}

// CONVERSIONS INTO NOTE ID
// ================================================================================================

impl From<&Note> for NoteId {
    fn from(note: &Note) -> Self {
        Self::new(note.recipient(), note.vault().hash())
    }
}

impl From<Word> for NoteId {
    fn from(value: Word) -> Self {
        Self(value.into())
    }
}

impl From<Digest> for NoteId {
    fn from(value: Digest) -> Self {
        Self(value)
    }
}

// CONVERSIONS FROM NOTE ID
// ================================================================================================

impl From<NoteId> for Word {
    fn from(id: NoteId) -> Self {
        id.0.into()
    }
}

impl From<NoteId> for [u8; 32] {
    fn from(id: NoteId) -> Self {
        id.0.into()
    }
}

impl From<&NoteId> for Word {
    fn from(id: &NoteId) -> Self {
        id.0.into()
    }
}

impl From<&NoteId> for [u8; 32] {
    fn from(id: &NoteId) -> Self {
        id.0.into()
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteId {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_bytes(&self.0.to_bytes());
    }
}

impl Deserializable for NoteId {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let id = Digest::read_from(source)?;
        Ok(Self(id))
    }
}
