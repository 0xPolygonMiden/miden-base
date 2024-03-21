use alloc::string::String;
use core::fmt::{Debug, Display, Formatter};

use super::{
    ByteReader, ByteWriter, Deserializable, DeserializationError, Digest, Felt, Hasher, Note,
    Serializable, Word, WORD_SIZE, ZERO,
};
use crate::utils::{hex_to_bytes, HexParseError};

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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
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

    /// Returns the most significant felt (the last element in array)
    pub fn most_significant_felt(&self) -> Felt {
        self.as_elements()[3]
    }

    /// Returns the digest defining this nullifier.
    pub fn inner(&self) -> Digest {
        self.0
    }

    /// Creates a Nullifier from a hex string. Assumes that the string starts with "0x" and
    /// that the hexadecimal characters are big-endian encoded.
    pub fn from_hex(hex_value: &str) -> Result<Self, HexParseError> {
        hex_to_bytes(hex_value).and_then(|bytes: [u8; 32]| {
            let digest = Digest::try_from(bytes)?;
            Ok(digest.into())
        })
    }

    /// Returns a big-endian, hex-encoded string.
    pub fn to_hex(&self) -> String {
        self.0.to_hex()
    }
}

impl Display for Nullifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl Debug for Nullifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self, f)
    }
}

// CONVERSIONS INTO NULLIFIER
// ================================================================================================

impl From<&Note> for Nullifier {
    fn from(note: &Note) -> Self {
        Self::new(
            note.script.hash(),
            note.inputs.commitment(),
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

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use crate::notes::Nullifier;

    #[test]
    fn test_from_hex_and_back() {
        let nullifier_hex = "0x41e7dbbc8ce63ec25cf2d76d76162f16ef8fd1195288171f5e5a3e178222f6d2";
        let nullifier = Nullifier::from_hex(nullifier_hex).unwrap();

        assert_eq!(nullifier_hex, nullifier.to_hex());
    }
}
