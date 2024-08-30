use alloc::vec::Vec;
use vm_core::{
    utils::{Deserializable, Serializable},
    Word, EMPTY_WORD, ZERO,
};
use vm_processor::DeserializationError;

use super::{map::EMPTY_STORAGE_MAP_ROOT, Felt, StorageMap};

mod r#type;
pub use r#type::StorageSlotType;

// STORAGE SLOT
// ================================================================================================

/// An object that represents the type of a storage slot.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageSlot {
    Value(Word),
    Map(StorageMap),
}

impl StorageSlot {
    /// Returns true if this storage slot has the default of this type.
    pub fn is_default(&self) -> bool {
        match self {
            StorageSlot::Value(value) => *value == EMPTY_WORD,
            StorageSlot::Map(map) => *map.root() == EMPTY_STORAGE_MAP_ROOT,
        }
    }

    /// Returns the empty [Word] for a value of this type.
    pub fn default_word(&self) -> Word {
        match self {
            StorageSlot::Value(_) => EMPTY_WORD,
            StorageSlot::Map(_) => EMPTY_STORAGE_MAP_ROOT,
        }
    }

    /// Returns the storage slot as field elements
    pub fn as_elements(&self) -> [Felt; 8] {
        self.into()
    }

    /// Returns the storage slot value as a [Word]
    pub fn get_value_as_word(&self) -> Word {
        match self {
            Self::Value(value) => *value,
            Self::Map(map) => {
                let mut word = [ZERO; 4];
                word.copy_from_slice(&map.root().as_elements());
                word
            },
        }
    }

    /// Returns the type for a certain storage slot
    pub fn get_slot_type(&self) -> StorageSlotType {
        match self {
            StorageSlot::Value(_) => StorageSlotType::Value,
            StorageSlot::Map(_) => StorageSlotType::Map,
        }
    }
}

impl Default for StorageSlot {
    fn default() -> Self {
        StorageSlot::Value(EMPTY_WORD)
    }
}

impl From<StorageSlot> for [Felt; 8] {
    fn from(value: StorageSlot) -> Self {
        let mut elements = [ZERO; 8];
        elements.copy_from_slice(&value.get_value_as_word());
        elements.copy_from_slice(&value.get_slot_type().as_word());
        elements
    }
}

impl From<&StorageSlot> for [Felt; 8] {
    fn from(value: &StorageSlot) -> Self {
        Self::from(value.clone())
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for StorageSlot {
    fn write_into<W: vm_core::utils::ByteWriter>(&self, target: &mut W) {
        target.write_u16(self.into());
    }
}

impl Deserializable for StorageSlot {
    fn read_from<R: vm_core::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, vm_processor::DeserializationError> {
        let encoded = source.read_u16()?;
        StorageSlotType::try_from(encoded).map_err(DeserializationError::InvalidValue)
    }
}
