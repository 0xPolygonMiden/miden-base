// STORAGE SLOT TYPE
// ================================================================================================

use vm_core::{Word, ONE, ZERO};

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
