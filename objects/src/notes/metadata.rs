use vm_processor::DeserializationError;

use super::{
    AccountId, ByteReader, ByteWriter, Deserializable, Felt, NoteError, Serializable, Word,
};

// NOTE METADATA
// ================================================================================================

/// Represents metadata associated with a note.
///
/// The metadata consists of:
/// - sender is the account which created the note.
/// - tag is a tag which can be used to identify the target account for the note.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteMetadata {
    sender: AccountId,
    tag: Felt,
}

impl NoteMetadata {
    /// Returns a new [NoteMetadata] instantiated with the specified parameters.
    pub fn new(sender: AccountId, tag: Felt) -> Self {
        Self { sender, tag }
    }

    /// Returns the account which created the note.
    pub fn sender(&self) -> AccountId {
        self.sender
    }

    /// Returns the tag associated with the note.
    pub fn tag(&self) -> Felt {
        self.tag
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
        elements
    }
}

impl TryFrom<Word> for NoteMetadata {
    type Error = NoteError;

    fn try_from(elements: Word) -> Result<Self, Self::Error> {
        Ok(Self {
            sender: elements[1].try_into().map_err(NoteError::NoteMetadataSenderInvalid)?,
            tag: elements[0],
        })
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteMetadata {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.sender.write_into(target);
        self.tag.write_into(target);
    }
}

impl Deserializable for NoteMetadata {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let sender = AccountId::read_from(source)?;
        let tag = Felt::read_from(source)?;

        Ok(Self { sender, tag })
    }
}
