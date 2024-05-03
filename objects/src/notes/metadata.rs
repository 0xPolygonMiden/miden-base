use alloc::string::ToString;

use super::{
    AccountId, ByteReader, ByteWriter, Deserializable, DeserializationError, Felt, NoteError,
    NoteTag, NoteType, Serializable, Word,
};

// NOTE METADATA
// ================================================================================================

/// Metadata associated with a note.
///
/// Note type and tag must be internally consistent according to the following rules:
///
/// - For off-chain notes, the most significant bit of the tag must be 0.
/// - For public notes, the second most significant bit of the tag must be 0.
/// - For encrypted notes, two most significant bits of the tag must be 00.
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteMetadata {
    /// The ID of the account which created the note.
    sender: AccountId,

    /// Defines how the note is to be stored (e.g., on-chain or off-chain).
    note_type: NoteType,

    /// A value which can be used by the recipient(s) to identify notes intended for them.
    tag: NoteTag,

    /// An arbitrary user-defined value.
    aux: Felt,
}

impl NoteMetadata {
    /// Returns a new [NoteMetadata] instantiated with the specified parameters.
    ///
    /// # Errors
    /// Returns an error if the note type and note tag are inconsistent.
    pub fn new(
        sender: AccountId,
        note_type: NoteType,
        tag: NoteTag,
        aux: Felt,
    ) -> Result<Self, NoteError> {
        let tag = tag.validate(note_type)?;
        Ok(Self { sender, note_type, tag, aux })
    }

    /// Returns the account which created the note.
    pub fn sender(&self) -> AccountId {
        self.sender
    }

    /// Returns the note's type.
    pub fn note_type(&self) -> NoteType {
        self.note_type
    }

    /// Returns the tag associated with the note.
    pub fn tag(&self) -> NoteTag {
        self.tag
    }

    /// Returns the note's aux field.
    pub fn aux(&self) -> Felt {
        self.aux
    }

    /// Returns `true` if the note is off-chain.
    pub fn is_offchain(&self) -> bool {
        self.note_type == NoteType::OffChain
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
        elements[0] = metadata.tag.inner().into();
        elements[1] = metadata.sender.into();
        elements[2] = metadata.note_type.into();
        elements[3] = metadata.aux;
        elements
    }
}

impl TryFrom<Word> for NoteMetadata {
    type Error = NoteError;

    fn try_from(elements: Word) -> Result<Self, Self::Error> {
        let sender = elements[1].try_into().map_err(NoteError::InvalidNoteSender)?;
        let note_type = elements[2].try_into()?;
        let tag: u64 = elements[0].into();
        let tag: u32 =
            tag.try_into().map_err(|_| NoteError::InconsistentNoteTag(note_type, tag))?;
        Self::new(sender, note_type, tag.into(), elements[3])
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteMetadata {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.sender.write_into(target);
        self.note_type.write_into(target);
        self.tag.write_into(target);
        self.aux.write_into(target);
    }
}

impl Deserializable for NoteMetadata {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let sender = AccountId::read_from(source)?;
        let note_type = NoteType::read_from(source)?;
        let tag = NoteTag::read_from(source)?;
        let aux = Felt::read_from(source)?;

        Self::new(sender, note_type, tag, aux)
            .map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}
