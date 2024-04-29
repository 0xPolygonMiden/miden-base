use core::fmt;

use miden_crypto::Felt;

use super::{
    note_type::{ENCRYPTED, OFF_CHAIN, PUBLIC},
    AccountId, ByteReader, ByteWriter, Deserializable, DeserializationError, NoteError, NoteType,
    Serializable,
};

// NOTE TAG
// ================================================================================================

// The higher two bits of the tag encode the note's type.
pub const NOTE_TYPE_MASK_SHIFT: u32 = 30;
pub const NOTE_TYPE_MASK: u32 = 0b11 << NOTE_TYPE_MASK_SHIFT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteTag(u32);

impl NoteTag {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new [NoteTag] instantiated from the specified account ID.
    pub fn from_account_id(account_id: AccountId, note_type: NoteType) -> Result<Self, NoteError> {
        let note_type_bits = (note_type as u32) << 30;
        let account_id_highbits = (u64::from(account_id) & 0xffff000000000000) >> 34;
        let tag = note_type_bits | (account_id_highbits as u32);

        Ok(NoteTag(tag))
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the inner u32 value of this tag.
    pub fn inner(&self) -> u32 {
        self.0
    }

    // UTILITY METHODS
    // --------------------------------------------------------------------------------------------

    /// Returns an error if this tag is not consistent with the specified note type, and self
    /// otherwise.
    pub fn validate(&self, note_type: NoteType) -> Result<Self, NoteError> {
        let encoded_type = (self.0 & NOTE_TYPE_MASK) >> NOTE_TYPE_MASK_SHIFT;
        if encoded_type != (note_type as u32) {
            Err(NoteError::InconsistentNoteTag(note_type, self.0 as u64))
        } else {
            Ok(*self)
        }
    }
}

impl fmt::Display for NoteTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// CONVERSIONS INTO NOTE TAG
// ================================================================================================

impl TryFrom<u32> for NoteTag {
    type Error = NoteError;

    fn try_from(tag: u32) -> Result<Self, Self::Error> {
        let note_type = (tag >> NOTE_TYPE_MASK_SHIFT) as u8;

        if note_type != PUBLIC && note_type != OFF_CHAIN && note_type != ENCRYPTED {
            return Err(NoteError::InvalidNoteTypeValue(tag.into()));
        }

        Ok(NoteTag(tag))
    }
}

impl TryFrom<u64> for NoteTag {
    type Error = NoteError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let tag: u32 = value.try_into().map_err(|_| NoteError::InvalidNoteTypeValue(value))?;
        tag.try_into()
    }
}

impl TryFrom<Felt> for NoteTag {
    type Error = NoteError;

    fn try_from(value: Felt) -> Result<Self, Self::Error> {
        value.as_int().try_into()
    }
}

impl From<NoteType> for NoteTag {
    fn from(value: NoteType) -> Self {
        NoteTag((value as u32) << NOTE_TYPE_MASK_SHIFT)
    }
}

// CONVERSIONS FROM NOTE TAG
// ================================================================================================

impl From<NoteTag> for u32 {
    fn from(value: NoteTag) -> Self {
        value.0
    }
}

impl From<NoteTag> for u64 {
    fn from(value: NoteTag) -> Self {
        value.0 as u64
    }
}

impl From<NoteTag> for Felt {
    fn from(value: NoteTag) -> Self {
        Felt::from(value.0)
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteTag {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.0.write_into(target);
    }
}

impl Deserializable for NoteTag {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let tag = u32::read_from(source)?;
        Ok(Self(tag))
    }
}

#[cfg(test)]
mod test {
    use miden_crypto::Felt;

    use super::NoteTag;
    use crate::notes::{NoteType, ENCRYPTED, NOTE_TYPE_MASK_SHIFT, OFF_CHAIN, PUBLIC};

    #[test]
    fn test_conversion() {
        let public = (PUBLIC as u32) << NOTE_TYPE_MASK_SHIFT;
        let off_chain = (OFF_CHAIN as u32) << NOTE_TYPE_MASK_SHIFT;
        let encrypted = (ENCRYPTED as u32) << NOTE_TYPE_MASK_SHIFT;

        assert_eq!(NoteTag::from(NoteType::Public), NoteTag(public));
        assert_eq!(NoteTag::from(NoteType::OffChain), NoteTag(off_chain));
        assert_eq!(NoteTag::from(NoteType::Encrypted), NoteTag(encrypted));

        assert_eq!(NoteTag::from(NoteType::Public), public.try_into().unwrap());
        assert_eq!(NoteTag::from(NoteType::OffChain), off_chain.try_into().unwrap());
        assert_eq!(NoteTag::from(NoteType::Encrypted), encrypted.try_into().unwrap());

        let public = u64::from(public);
        let off_chain = u64::from(off_chain);
        let encrypted = u64::from(encrypted);

        assert_eq!(NoteTag::from(NoteType::Public), public.try_into().unwrap());
        assert_eq!(NoteTag::from(NoteType::OffChain), off_chain.try_into().unwrap());
        assert_eq!(NoteTag::from(NoteType::Encrypted), encrypted.try_into().unwrap());

        assert_eq!(NoteTag::from(NoteType::Public), Felt::new(public).try_into().unwrap());
        assert_eq!(NoteTag::from(NoteType::OffChain), Felt::new(off_chain).try_into().unwrap());
        assert_eq!(NoteTag::from(NoteType::Encrypted), Felt::new(encrypted).try_into().unwrap());
    }

    #[test]
    fn test_validation() {
        assert!(NoteTag::from(NoteType::Public).validate(NoteType::Public).is_ok());
        assert!(NoteTag::from(NoteType::OffChain).validate(NoteType::OffChain).is_ok());
        assert!(NoteTag::from(NoteType::Encrypted).validate(NoteType::Encrypted).is_ok());

        assert!(NoteTag::from(NoteType::Public).validate(NoteType::OffChain).is_err());
        assert!(NoteTag::from(NoteType::Public).validate(NoteType::Encrypted).is_err());
        assert!(NoteTag::from(NoteType::OffChain).validate(NoteType::Public).is_err());
        assert!(NoteTag::from(NoteType::OffChain).validate(NoteType::Encrypted).is_err());
        assert!(NoteTag::from(NoteType::Encrypted).validate(NoteType::OffChain).is_err());
        assert!(NoteTag::from(NoteType::Encrypted).validate(NoteType::Public).is_err());
    }
}
