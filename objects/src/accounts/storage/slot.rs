use super::{AccountError, Felt, StorageEntry, StorageEntryType, Word};

/// An object that represents a storage slot.
///
/// The storage slot consists of a storage slot type which describes the type of data the slot
/// contains. It also contains the storage entry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct StorageSlot {
    slot_type: StorageSlotType,
    entry: StorageEntry,
}

impl Default for StorageSlot {
    fn default() -> Self {
        Self {
            slot_type: StorageSlotType::Scalar(StorageEntryType::Scalar),
            entry: StorageEntry::Scalar(Word::default()),
        }
    }
}

impl StorageSlot {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// The minimum depth of an array slot.
    const MIN_ARRAY_DEPTH: u8 = 2;

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new scalar [StorageSlot] instance with the provided entry.
    pub fn new_scalar(entry: StorageEntry) -> Self {
        Self {
            slot_type: StorageSlotType::Scalar(entry.entry_type()),
            entry,
        }
    }

    /// Returns a new map [StorageSlot] instance with the provided commitment and entry type.
    pub fn new_map(commitment: Word, entry_type: StorageEntryType) -> Self {
        Self {
            slot_type: StorageSlotType::Map(entry_type),
            entry: StorageEntry::Scalar(commitment),
        }
    }

    /// Returns a new array [StorageSlot] instance with the provided depth, commitment, and entry
    /// type.
    pub fn new_array(
        depth: u8,
        commitment: Word,
        entry_type: StorageEntryType,
    ) -> Result<Self, AccountError> {
        if depth < 2 {
            return Err(AccountError::StorageSlotArrayTooSmall {
                actual: depth,
                min: Self::MIN_ARRAY_DEPTH,
            });
        }
        Ok(Self {
            slot_type: StorageSlotType::Array { depth, entry_type },
            entry: StorageEntry::Scalar(commitment),
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the type of this slot.
    pub fn slot_type(&self) -> &StorageSlotType {
        &self.slot_type
    }

    /// Returns the entry of this slot.
    pub fn entry(&self) -> &StorageEntry {
        &self.entry
    }

    // CONSUMERS
    // --------------------------------------------------------------------------------------------

    /// Consumes this slot and returns the underlying slot type and entry.
    pub fn into_inner(self) -> (StorageSlotType, StorageEntry) {
        (self.slot_type, self.entry)
    }
}

/// An object that represents the type of a storage slot.
///
/// The type of the value in a storage entry is described by the [ValueType] object.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum StorageSlotType {
    /// Represents a slot that contains a scalar slot with entry of type [StorageEntryType].
    Scalar(StorageEntryType),
    /// Represents a slot that contains a commitment to a map with entries of type [StorageEntryType].
    Map(StorageEntryType),
    /// Represents a slot that contains a commitment to an array with capacity 2^depth with entries
    /// of type [StorageEntryType].
    Array {
        depth: u8,
        entry_type: StorageEntryType,
    },
}

impl From<&StorageSlotType> for Felt {
    fn from(slot_type: &StorageSlotType) -> Self {
        match slot_type {
            StorageSlotType::Scalar(entry_type) => {
                let type_value = (u8::from(entry_type) as u64) << 32;
                Felt::from(type_value)
            }
            StorageSlotType::Map(entry_type) => {
                let type_value = ((u8::from(entry_type) as u64) << 32) | 1_u64;
                Felt::from(type_value)
            }
            StorageSlotType::Array { depth, entry_type } => {
                let type_value = ((u8::from(entry_type) as u64) << 32) | (*depth as u64);
                Felt::from(type_value)
            }
        }
    }
}
