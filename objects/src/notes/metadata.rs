use vm_processor::DeserializationError;

use super::{
    AccountId, ByteReader, ByteWriter, Deserializable, Felt, NoteError, NoteType, Serializable,
    Word,
};

// NOTE METADATA
// ================================================================================================

/// Represents metadata associated with a note.
///
/// The metadata consists of:
/// - sender is the account which created the note.
/// - tag is a value which can be used by the recipient(s) to identify notes intended for them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteMetadata {
    sender: AccountId,
    tag: Felt,
    note_type: NoteType,
    aux: Felt,
}

impl NoteMetadata {
    /// Returns a new [NoteMetadata] instantiated with the specified parameters.
    pub fn new(
        sender: AccountId,
        note_type: NoteType,
        tag: Felt,
        aux: Felt,
    ) -> Result<Self, NoteError> {
        match note_type {
            NoteType::OffChain => {
                // 1st most significant bit set to 0
                if tag.as_int() >> 63 != 0 {
                    return Err(NoteError::InvalidOffchainTag);
                }
            },
            NoteType::Public => {
                // 2nd most significant bit set to 0
                if (tag.as_int() >> 62) & 1 != 0 {
                    return Err(NoteError::InvalidPublicTag);
                }
            },
            NoteType::Encrypted => {
                // 1st & 2nd most significant bit set to 0
                if tag.as_int() >> 62 != 0 {
                    return Err(NoteError::InvalidEncryptedTag);
                }
            },
        }

        Ok(Self { sender, tag, note_type, aux })
    }

    /// Returns the account which created the note.
    pub fn sender(&self) -> AccountId {
        self.sender
    }

    /// Returns the tag associated with the note.
    pub fn tag(&self) -> Felt {
        self.tag
    }

    /// Returns the note's type.
    pub fn note_type(&self) -> NoteType {
        self.note_type
    }
}

impl From<NoteMetadata> for Word {
    fn from(metadata: NoteMetadata) -> Self {
        (&metadata).into()
    }
}

impl From<&NoteMetadata> for Word {
    fn from(metadata: &NoteMetadata) -> Self {
        let mut elements = Word::default();
        elements[0] = metadata.tag;
        elements[1] = metadata.sender.into();
        elements[2] = metadata.note_type().into();
        elements
    }
}

impl TryFrom<Word> for NoteMetadata {
    type Error = NoteError;

    fn try_from(elements: Word) -> Result<Self, Self::Error> {
        Ok(Self {
            sender: elements[1].try_into().map_err(NoteError::NoteMetadataSenderInvalid)?,
            tag: elements[0],
            note_type: elements[2].try_into()?,
            aux: elements[3],
        })
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteMetadata {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.sender.write_into(target);
        self.tag.write_into(target);
        self.note_type.write_into(target);
        self.aux.write_into(target);
    }
}

impl Deserializable for NoteMetadata {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let sender = AccountId::read_from(source)?;
        let tag = Felt::read_from(source)?;
        let note_type = NoteType::read_from(source)?;
        let aux = Felt::read_from(source)?;

        Ok(Self { sender, tag, note_type, aux })
    }
}
