use vm_core::{
    utils::{Deserializable, Serializable},
    Word, EMPTY_WORD,
};

use super::{map::EMPTY_STORAGE_MAP_ROOT, StorageMap};

// STORAGE SLOT
// ================================================================================================

/// An enum that represents the type of a storage slot and it's value.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageSlot {
    Value(Word),
    Map(StorageMap),
}

impl StorageSlot {
    // CONSTANTS
    // ================================================================================================

    /// Returns the empty [Word] for a value of this type.
    pub fn default_word(&self) -> Word {
        match self {
            StorageSlot::Value(_) => EMPTY_WORD,
            StorageSlot::Map(_) => EMPTY_STORAGE_MAP_ROOT,
        }
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for StorageSlot {
    fn write_into<W: vm_core::utils::ByteWriter>(&self, target: &mut W) {
        target.write(self);
    }
}

impl Deserializable for StorageSlot {
    fn read_from<R: vm_core::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, vm_processor::DeserializationError> {
        let storage_slot: StorageSlot = source.read()?;
        Ok(storage_slot)
    }
}
