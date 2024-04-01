use core::fmt;
use std::num::TryFromIntError;

use super::{
    AccountId, ByteReader, ByteWriter, Deserializable, DeserializationError, NoteError,
    NoteExecutionMode, NoteType, Serializable,
};

// NOTE TAG
// ================================================================================================

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteTag(u32);

impl NoteTag {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new [NoteTag] instantiated from the specified account ID.
    ///
    /// The tag is constructed as follows:
    /// - For local execution, the two most significant bits are set to 0b00, the following 16 bits
    ///   are set to the 16 most significant bits of the account ID, and the remaining 14 bits are
    ///   set to 0.
    /// - For network execution, the two most significant bits are set to 0b10 and the remaining
    ///   bits are set to the 30 most significant bits of the account ID.
    pub fn from_account_id(
        account_id: AccountId,
        execution: NoteExecutionMode,
    ) -> Result<Self, NoteError> {
        match execution {
            NoteExecutionMode::Local => {
                let id: u64 = account_id.into();
                // select the 16 high bits of the account id
                let high_bits = id & 0xffff000000000000;
                // set bits (30,14] with the account id data
                // set bits (32,30] as `0b00` identifying the note as intended for local execution
                Ok(Self((high_bits >> 34) as u32))
            },
            NoteExecutionMode::Network => {
                if !account_id.is_on_chain() {
                    Err(NoteError::NetworkExecutionRequiresOnChainAccount)
                } else {
                    let id: u64 = account_id.into();
                    // select the 30 high bits of the account id
                    let high_bits = id & 0xfffffffc00000000;
                    // set bits (30,0] with the account id data
                    let tag = (high_bits >> 34) as u32;
                    // set bits (32,30] as `0b10` identifying the note as intended for network
                    // execution
                    Ok(Self(tag | 0x80000000))
                }
            },
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns true if the note is intended for execution by a specific account.
    ///
    /// A note is intended for execution by a single account if either the first two bits are zeros
    /// or the first 3 bits are 0b100.
    pub fn is_single_target(&self) -> bool {
        let first_2_bit = self.0 >> 30;
        let first_3_bits = self.0 >> 29;
        first_2_bit == 0b00 || first_3_bits == 0b100
    }

    /// Returns note execution mode defined by this tag.
    ///
    /// If the most significant bit of the tag is 0 or the 3 most significant bits are equal to
    /// 0b101, the note is intended for local execution; otherwise, the note is intended for
    /// network execution.
    pub fn execution_mode(&self) -> NoteExecutionMode {
        let first_bit = self.0 >> 31;
        let first_3_bits = self.0 >> 29;

        if first_bit == 0 || first_3_bits == 0b101 {
            NoteExecutionMode::Local
        } else {
            NoteExecutionMode::Network
        }
    }

    /// Returns the inner u32 value of this tag.
    pub fn inner(&self) -> u32 {
        self.0
    }

    // UTILITY METHODS
    // --------------------------------------------------------------------------------------------

    /// Returns an error if this tag is not consistent with the specified note type, and self
    /// otherwise.
    ///
    /// The tag and the note type are consistent if they satisfy the following rules:
    /// - For off-chain notes, the most significant bit of the tag is 0.
    /// - For public notes, the second most significant bit of the tag is 0.
    /// - For encrypted notes, two most significant bits of the tag is 00.
    pub fn validate(&self, note_type: NoteType) -> Result<Self, NoteError> {
        let tag_mask = note_type as u32;
        if (self.0 >> 30) & tag_mask != 0 {
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

impl From<u32> for NoteTag {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<NoteTag> for u32 {
    fn from(value: NoteTag) -> Self {
        value.0
    }
}

impl TryFrom<u64> for NoteTag {
    type Error = TryFromIntError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Ok(Self(value.try_into()?))
    }
}

impl From<NoteTag> for u64 {
    fn from(value: NoteTag) -> Self {
        value.0 as u64
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
