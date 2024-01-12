use super::{Digest, Felt, Hasher, Note, Word, WORD_SIZE, ZERO};
use crate::utils::serde::{
    ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable,
};

// NULLIFIER
// ================================================================================================

/// A note's nullifier.
///
/// A note's nullifier is computed as hash(serial_num, script_hash, input_hash, asset_hash).
///
/// This achieves the following properties:
/// - Every note can be reduced to a single unique nullifier.
/// - We cannot derive a note's hash from its nullifier, or a note's nullifier from its hash.
/// - To compute the nullifier we must know all components of the note: serial_num, script_hash,
///   input_hash and asset_hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nullifier(Digest);

impl Nullifier {
    /// Returns a new note [Nullifier] instantiated from the provided digest.
    pub fn new(
        script_hash: Digest,
        inputs_hash: Digest,
        asset_hash: Digest,
        serial_num: Word,
    ) -> Self {
        let mut elements = [ZERO; 4 * WORD_SIZE];
        elements[..4].copy_from_slice(&serial_num);
        elements[4..8].copy_from_slice(script_hash.as_elements());
        elements[8..12].copy_from_slice(inputs_hash.as_elements());
        elements[12..].copy_from_slice(asset_hash.as_elements());
        Self(Hasher::hash_elements(&elements))
    }

    /// Returns the elements of this nullifier.
    pub fn as_elements(&self) -> &[Felt] {
        self.0.as_elements()
    }

    /// Returns the digest defining this nullifier.
    pub fn inner(&self) -> Digest {
        self.0
    }
}

// CONVERSIONS INTO NULLIFIER
// ================================================================================================

impl From<&Note> for Nullifier {
    fn from(note: &Note) -> Self {
        Self::new(
            note.script.hash(),
            note.inputs.hash(),
            note.assets.commitment(),
            note.serial_num,
        )
    }
}

impl From<Word> for Nullifier {
    fn from(value: Word) -> Self {
        Self(value.into())
    }
}

impl From<Digest> for Nullifier {
    fn from(value: Digest) -> Self {
        Self(value)
    }
}

// CONVERSIONS FROM NULLIFIER
// ================================================================================================

impl From<Nullifier> for Word {
    fn from(nullifier: Nullifier) -> Self {
        nullifier.0.into()
    }
}

impl From<Nullifier> for [u8; 32] {
    fn from(nullifier: Nullifier) -> Self {
        nullifier.0.into()
    }
}

impl From<&Nullifier> for Word {
    fn from(nullifier: &Nullifier) -> Self {
        nullifier.0.into()
    }
}

impl From<&Nullifier> for [u8; 32] {
    fn from(nullifier: &Nullifier) -> Self {
        nullifier.0.into()
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for Nullifier {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_bytes(&self.0.to_bytes());
    }
}

impl Deserializable for Nullifier {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let nullifier = Digest::read_from(source)?;
        Ok(Self(nullifier))
    }
}
