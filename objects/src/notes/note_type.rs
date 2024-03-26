use crate::{
    utils::{
        format,
        serde::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    },
    Felt, NoteError,
};

// CONSTANTS
// ================================================================================================
const OFF_CHAIN: u8 = 0;
const ENCRYPTED: u8 = 1;
const PUBLIC: u8 = 2;

// NOTE TYPE
// ================================================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[repr(u8)]
pub enum NoteType {
    /// Notes with this type have only their hash published to the network.
    OffChain = OFF_CHAIN,

    /// Notes with type are shared with the network encrypted.
    Encrypted = ENCRYPTED,

    /// Notes with this type are fully shared with the network.
    Public = PUBLIC,
}

impl From<NoteType> for Felt {
    fn from(id: NoteType) -> Self {
        Felt::new(id as u64)
    }
}

impl TryFrom<Felt> for NoteType {
    type Error = NoteError;

    fn try_from(value: Felt) -> Result<Self, Self::Error> {
        let value = value.as_int();
        let note_type: u8 = value.try_into().map_err(|_| NoteError::NoteTypeInvalid(value))?;
        match note_type {
            OFF_CHAIN => Ok(NoteType::OffChain),
            ENCRYPTED => Ok(NoteType::Encrypted),
            PUBLIC => Ok(NoteType::Public),
            _ => Err(NoteError::NoteTypeInvalid(value)),
        }
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteType {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        (*self as u8).write_into(target)
    }
}

impl Deserializable for NoteType {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let discriminat = u8::read_from(source)?;

        let note_type = match discriminat {
            OFF_CHAIN => NoteType::OffChain,
            ENCRYPTED => NoteType::Encrypted,
            PUBLIC => NoteType::Public,
            v => {
                return Err(DeserializationError::InvalidValue(format!(
                    "Value {} is not a valid NoteType",
                    v
                )))
            },
        };

        Ok(note_type)
    }
}
