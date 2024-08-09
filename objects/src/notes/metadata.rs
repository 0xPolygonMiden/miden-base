use alloc::string::ToString;

use super::{
    execution_hint::NoteExecutionHint, AccountId, ByteReader, ByteWriter, Deserializable,
    DeserializationError, Felt, NoteError, NoteTag, NoteType, Serializable, Word,
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

    /// Specifies when a note is ready to be consumed.
    execution_hint: NoteExecutionHint,
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
        execution_hint: NoteExecutionHint,
        aux: Felt,
    ) -> Result<Self, NoteError> {
        let tag = tag.validate(note_type)?;
        Ok(Self {
            sender,
            note_type,
            tag,
            aux,
            execution_hint,
        })
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

    /// Returns the execution hint associated with the note.
    pub fn execution_hint(&self) -> NoteExecutionHint {
        self.execution_hint
    }

    /// Returns the note's aux field.
    pub fn aux(&self) -> Felt {
        self.aux
    }

    /// Returns `true` if the note is private.
    pub fn is_private(&self) -> bool {
        self.note_type == NoteType::Private
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
        elements[0] = metadata.aux;
        elements[1] = Felt::new(merge_type_and_hint(metadata.note_type, metadata.execution_hint));
        elements[2] = metadata.sender.into();
        elements[3] = metadata.tag.inner().into();
        elements
    }
}

impl TryFrom<Word> for NoteMetadata {
    type Error = NoteError;

    fn try_from(elements: Word) -> Result<Self, Self::Error> {
        let (note_type, note_execution_hint) = unmerge_type_and_hint(elements[1].into())?;
        let sender = elements[2].try_into().map_err(NoteError::InvalidNoteSender)?;
        let tag: u64 = elements[3].into();
        let tag: u32 =
            tag.try_into().map_err(|_| NoteError::InconsistentNoteTag(note_type, tag))?;

        Self::new(sender, note_type, tag.into(), note_execution_hint, elements[0])
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteMetadata {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.sender.write_into(target);
        target.write_u64(merge_type_and_hint(self.note_type, self.execution_hint));
        self.tag.write_into(target);
        self.aux.write_into(target);
    }
}

impl Deserializable for NoteMetadata {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let sender = AccountId::read_from(source)?;
        let (note_type, note_execution_hint) = unmerge_type_and_hint(source.read_u64()?)
            .map_err(|err| DeserializationError::InvalidValue(err.to_string()))?;
        let tag = NoteTag::read_from(source)?;
        let aux = Felt::read_from(source)?;

        Self::new(sender, note_type, tag, note_execution_hint, aux)
            .map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Encodes `note_type` and `note_execution_hint` into a [u64] such that the resulting number has
/// the following structure (from most significant bit to the least significant bit):
///
/// - Bits 39 to 38 (2 bits): NoteType
/// - Bits 37 to 32 (6 bits): NoteExecutionHint tag
/// - Bits 31 to 0 (32 bits): NoteExecutionHint payload
pub fn merge_type_and_hint(note_type: NoteType, note_execution_hint: NoteExecutionHint) -> u64 {
    let type_nibble = note_type as u64 & 0b11;
    let (tag_nibble, payload_u32) = note_execution_hint.into_parts();

    let tag_section = (tag_nibble as u64) & 0b111111;
    let payload_section = payload_u32 as u64;

    (type_nibble << 38) | (tag_section << 32) | payload_section
}

pub fn unmerge_type_and_hint(value: u64) -> Result<(NoteType, NoteExecutionHint), NoteError> {
    let high_nibble = ((value >> 38) & 0b11) as u8;
    let tag_byte = ((value >> 32) & 0b111111) as u8;
    let payload_u32 = (value & 0xFFFFFFFF) as u32;

    let note_type = NoteType::try_from(high_nibble)?;
    let note_execution_hint = NoteExecutionHint::from_parts(tag_byte, payload_u32)?;

    Ok((note_type, note_execution_hint))
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_and_unmerge() {
        let note_type = NoteType::Public;
        let note_execution_hint = NoteExecutionHint::OnBlockSlot {
            epoch_len: 10,
            slot_len: 11,
            slot_offset: 12,
        };

        let merged_value = merge_type_and_hint(note_type, note_execution_hint);
        let (extracted_note_type, extracted_note_execution_hint) =
            unmerge_type_and_hint(merged_value).unwrap();

        assert_eq!(note_type, extracted_note_type);
        assert_eq!(note_execution_hint, extracted_note_execution_hint);

        let note_type = NoteType::Private;
        let note_execution_hint = NoteExecutionHint::Always;

        let merged_value = merge_type_and_hint(note_type, note_execution_hint);
        let (extracted_note_type, extracted_note_execution_hint) =
            unmerge_type_and_hint(merged_value).unwrap();

        assert_eq!(note_type, extracted_note_type);
        assert_eq!(note_execution_hint, extracted_note_execution_hint);
    }
}
