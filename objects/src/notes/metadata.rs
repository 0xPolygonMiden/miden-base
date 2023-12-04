use miden_crypto::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::DeserializationError;

use super::{AccountId, Felt, NoteError, Word};

/// Represents metadata associated with a note. This includes the sender, tag, and number of assets.
/// - sender is the account which created the note.
/// - tag is a tag which can be used to identify the target account for the note.
/// - num_assets is the number of assets in the note.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteMetadata {
    sender: AccountId,
    tag: Felt,
    num_assets: Felt,
}

impl NoteMetadata {
    /// Returns a new note metadata object created with the specified parameters.
    pub fn new(sender: AccountId, tag: Felt, num_assets: Felt) -> Self {
        // TODO: Assert num assets is valid
        Self {
            sender,
            tag,
            num_assets,
        }
    }

    /// Returns the account which created the note.
    pub fn sender(&self) -> AccountId {
        self.sender
    }

    /// Returns the tag associated with the note.
    pub fn tag(&self) -> Felt {
        self.tag
    }

    /// Returns the number of assets in the note.
    pub fn num_assets(&self) -> Felt {
        self.num_assets
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
        elements[0] = metadata.num_assets;
        elements[1] = metadata.tag;
        elements[2] = metadata.sender.into();
        elements
    }
}

impl TryFrom<Word> for NoteMetadata {
    type Error = NoteError;

    fn try_from(elements: Word) -> Result<Self, Self::Error> {
        // TODO: Assert num assets is valid
        Ok(Self {
            sender: elements[2].try_into().map_err(NoteError::NoteMetadataSenderInvalid)?,
            tag: elements[1],
            num_assets: elements[0],
        })
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteMetadata {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.sender.write_into(target);
        self.tag.write_into(target);
        self.num_assets.write_into(target);
    }
}

impl Deserializable for NoteMetadata {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let sender = AccountId::read_from(source)?;
        let tag = Felt::read_from(source)?;
        let num_assets = Felt::read_from(source)?;

        Ok(Self {
            sender,
            tag,
            num_assets,
        })
    }
}
