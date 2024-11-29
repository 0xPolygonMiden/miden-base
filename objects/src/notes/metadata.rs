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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
        elements[0] = metadata.sender.first_felt();
        elements[1] = merge_id_type_and_hint_tag(
            metadata.sender.second_felt(),
            metadata.note_type,
            metadata.execution_hint,
        );
        elements[2] = merge_note_tag_and_hint_payload(metadata.execution_hint, metadata.tag);
        elements[3] = metadata.aux;
        elements
    }
}

impl TryFrom<Word> for NoteMetadata {
    type Error = NoteError;

    fn try_from(elements: Word) -> Result<Self, Self::Error> {
        let sender_id_first_felt: Felt = elements[0];

        let (sender_id_second_felt, note_type, execution_hint_tag) =
            unmerge_id_type_and_hint_tag(elements[1])?;

        let sender = AccountId::try_from([sender_id_first_felt, sender_id_second_felt])
            .map_err(NoteError::NoteSenderInvalidAccountId)?;

        let (execution_hint, note_tag) =
            unmerge_note_tag_and_hint_payload(elements[2], execution_hint_tag)?;

        Self::new(sender, note_type, note_tag, execution_hint, elements[3])
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteMetadata {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        // TODO: Do we need a serialization format that is different from the Word encoding? It was
        // previously different.
        Word::from(self).write_into(target);
    }
}

impl Deserializable for NoteMetadata {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let word = Word::read_from(source)?;
        Self::try_from(word).map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Merges the second felt of an [`AccountId`], a [`NoteType`] and the tag of a
/// [`NoteExecutionHint`] into a single [`Felt`].
///
/// The layout is as follows:
///
/// ```text
/// [account_id_second_felt (56 bits) | note_type (3 bits) | note_execution_hint_tag (5 bits)]
/// ```
fn merge_id_type_and_hint_tag(
    sender_id_second_felt: Felt,
    note_type: NoteType,
    note_execution_hint: NoteExecutionHint,
) -> Felt {
    let mut merged = sender_id_second_felt.as_int().to_be_bytes();
    let type_bits = note_type as u8;
    let (tag_bits, _) = note_execution_hint.into_parts();

    debug_assert!(type_bits & 0b1111_1100 == 0, "note type must not contain values >= 4");
    debug_assert!(
        tag_bits & 0b1100_0000 == 0,
        "note execution hint tag must not contain values >= 64"
    );

    // Note: The 8th byte of the second AccountId felt is zero by construction.
    merged[7] |= type_bits << 6;
    merged[7] |= tag_bits;

    // SAFETY: One of the top 16 bits of the second felt is zero by construction so the bytes will
    // be a valid felt.
    Felt::try_from(merged.as_slice()).expect("encoded value should be a valid felt")
}

fn unmerge_id_type_and_hint_tag(element: Felt) -> Result<(Felt, NoteType, u8), NoteError> {
    let element = element.as_int();

    let sender_id_second_felt = element & 0xffff_ffff_ffff_ff00;
    let least_significant_byte = (element & 0xff) as u8;
    let note_type_bits = least_significant_byte & 0b1100_0000;
    let tag_bits = least_significant_byte & 0b0011_1111;

    let note_type = NoteType::try_from(note_type_bits)?;

    // SAFETY: The input element was valid and and we cleared additional bits and did not set any
    // bits, so it must still be a valid felt.
    let sender_id_second_felt =
        Felt::try_from(sender_id_second_felt).expect("element should still be valid");

    Ok((sender_id_second_felt, note_type, tag_bits))
}

/// Merges the [`NoteExecutionHint`] payload and a [`NoteTag`] into a single [`Felt`].
///
/// The layout is as follows:
///
/// ```text
/// [note_execution_hint_payload (32 bits) | note_tag (32 bits)]
/// ```
fn merge_note_tag_and_hint_payload(
    note_execution_hint: NoteExecutionHint,
    note_tag: NoteTag,
) -> Felt {
    let (_, payload) = note_execution_hint.into_parts();
    let note_tag: u32 = note_tag.into();

    let felt_bytes = ((payload as u64) << 32) | (note_tag as u64);

    // SAFETY: The payload is guaranteed to never be u32::MAX so at least one of the upper 32 bits
    // is zero, hence the felt is valid even if note_tag u32::MAX.
    Felt::try_from(felt_bytes).expect("bytes should be a valid felt")
}

fn unmerge_note_tag_and_hint_payload(
    element: Felt,
    note_execution_hint_tag: u8,
) -> Result<(NoteExecutionHint, NoteTag), NoteError> {
    let element = element.as_int();

    let payload = (element >> 32) as u32;
    let note_tag = (element & 0xffff_ffff) as u32;

    let execution_hint = NoteExecutionHint::from_parts(note_execution_hint_tag, payload)?;
    let note_tag = NoteTag::from(note_tag);

    Ok((execution_hint, note_tag))
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {

    // use super::*;

    // TODO: Refactor once decided whether we go with full or prefix account ID.
    // #[test]
    // fn test_merge_and_unmerge() {
    //     let note_type = NoteType::Public;
    //     let note_execution_hint = NoteExecutionHint::OnBlockSlot {
    //         epoch_len: 10,
    //         slot_len: 11,
    //         slot_offset: 12,
    //     };

    //     let merged_value = merge_type_and_hint(note_type, note_execution_hint);
    //     let (extracted_note_type, extracted_note_execution_hint) =
    //         unmerge_type_and_hint(merged_value).unwrap();

    //     assert_eq!(note_type, extracted_note_type);
    //     assert_eq!(note_execution_hint, extracted_note_execution_hint);

    //     let note_type = NoteType::Private;
    //     let note_execution_hint = NoteExecutionHint::Always;

    //     let merged_value = merge_type_and_hint(note_type, note_execution_hint);
    //     let (extracted_note_type, extracted_note_execution_hint) =
    //         unmerge_type_and_hint(merged_value).unwrap();

    //     assert_eq!(note_type, extracted_note_type);
    //     assert_eq!(note_execution_hint, extracted_note_execution_hint);

    //     let note_type = NoteType::Private;
    //     let note_execution_hint = NoteExecutionHint::None;

    //     let merged_value = merge_type_and_hint(note_type, note_execution_hint);
    //     let (extracted_note_type, extracted_note_execution_hint) =
    //         unmerge_type_and_hint(merged_value).unwrap();
    //     assert_eq!(note_type, extracted_note_type);
    //     assert_eq!(note_execution_hint, extracted_note_execution_hint);
    // }
}
