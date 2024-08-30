// STORAGE SLOT TYPE
// ================================================================================================

use alloc::string::ToString;

use vm_core::{
    utils::{ByteReader, ByteWriter, Deserializable, Serializable},
    Word, ONE, ZERO,
};
use vm_processor::DeserializationError;

/// An object that represents the type of a storage slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageSlotType {
    /// Represents a slot that contains a value.
    Value,
    /// Represents a slot that contains a commitment to a map with values.
    Map,
}

impl StorageSlotType {
    /// Returns storage slot type as a [Word]
    pub fn as_word(&self) -> Word {
        match self {
            StorageSlotType::Value => [ZERO, ZERO, ZERO, ZERO],
            StorageSlotType::Map => [ONE, ZERO, ZERO, ZERO],
        }
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for StorageSlotType {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            Self::Value { .. } => target.write_u8(0),
            Self::Map { .. } => target.write_u8(1),
        }
    }
}

impl Deserializable for StorageSlotType {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let storage_slot_type = source.read_u8()?;

        match storage_slot_type {
            0 => Ok(Self::Value),
            1 => Ok(Self::Map),
            _ => Err(DeserializationError::InvalidValue(storage_slot_type.to_string())),
        }
    }
}
