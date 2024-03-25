use alloc::string::{String, ToString};

use super::Felt;

// CONSTANTS
// ================================================================================================

const MAX_VALUE_ARITY: u8 = u8::MAX - 1;

const MIN_ARRAY_DEPTH: u8 = 2;
const MAX_ARRAY_DEPTH: u8 = 64;

const DEFAULT_SLOT_TYPE: StorageSlotType = StorageSlotType::Value { value_arity: 0 };

// STORAGE SLOT TYPE
// ================================================================================================

/// An object that represents the type of a storage slot.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StorageSlotType {
    /// Represents a slot that contains a value with the specified arity.
    Value { value_arity: u8 },
    /// Represents a slot that contains a commitment to a map with values with the specified arity.
    Map { value_arity: u8 },
    /// Represents a slot that contains a commitment to an array with capacity 2^depth with values
    /// with the specified arity.
    Array { depth: u8, value_arity: u8 },
}

impl StorageSlotType {
    /// Returns true if this storage slot type is valid.
    ///
    /// Valid storage slot types are defined as follows:
    /// - value arity must be between 0 and 254 (inclusive).
    /// - for Array types, depth must be between 2 and 64 (inclusive).
    pub fn is_valid(&self) -> bool {
        match self {
            StorageSlotType::Value { value_arity } => *value_arity <= MAX_VALUE_ARITY,
            StorageSlotType::Map { value_arity } => *value_arity <= MAX_VALUE_ARITY,
            StorageSlotType::Array { depth, value_arity } => {
                *value_arity < MAX_VALUE_ARITY
                    && *depth >= MIN_ARRAY_DEPTH
                    && *depth <= MAX_ARRAY_DEPTH
            },
        }
    }

    /// Returns true if this storage slot type is a value type with arity 0.
    pub fn is_default(&self) -> bool {
        match self {
            StorageSlotType::Value { value_arity } => *value_arity == 0,
            _ => false,
        }
    }
}

impl Default for StorageSlotType {
    fn default() -> Self {
        DEFAULT_SLOT_TYPE
    }
}

// CONVERSIONS INTO STORAGE SLOT TYPE
// ================================================================================================

impl TryFrom<u16> for StorageSlotType {
    type Error = String;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        let data_type = value as u8;
        let value_arity = (value >> 8) as u8;

        if value_arity > MAX_VALUE_ARITY {
            return Err("Invalid value arity".to_string());
        }

        match data_type {
            0 => Ok(StorageSlotType::Value { value_arity }),
            1 => Ok(StorageSlotType::Map { value_arity }),
            2..=MAX_ARRAY_DEPTH => Ok(StorageSlotType::Array { depth: data_type, value_arity }),
            _ => Err("invalid slot data type".to_string()),
        }
    }
}

// CONVERSIONS FROM STORAGE SLOT TYPE
// ================================================================================================

impl From<StorageSlotType> for u16 {
    /// Converts storage type into a u16 value as the following 2 bytes:
    ///
    /// [value_arity, slot_data_type]
    ///
    /// Where slot_data_type is 0 for Value type, 1 for Map type, and set to depth fro Array type.
    fn from(slot_type: StorageSlotType) -> Self {
        match slot_type {
            StorageSlotType::Value { value_arity } => (value_arity as u16) << 8,
            StorageSlotType::Map { value_arity } => ((value_arity as u16) << 8) | 1_u16,
            StorageSlotType::Array { depth, value_arity } => {
                ((value_arity as u16) << 8) | (depth as u16)
            },
        }
    }
}

impl From<&StorageSlotType> for u16 {
    fn from(value: &StorageSlotType) -> Self {
        Self::from(*value)
    }
}

impl From<StorageSlotType> for Felt {
    /// Converts storage type into a field element as the following 32-bit values:
    ///
    /// [value_arity, slot_data_type]
    ///
    /// Where slot_data_type is 0 for Value type, 1 for Map type, and set to depth fro Array type.
    fn from(slot_type: StorageSlotType) -> Self {
        match slot_type {
            StorageSlotType::Value { value_arity } => {
                let type_value = (value_arity as u64) << 32;
                Felt::new(type_value)
            },
            StorageSlotType::Map { value_arity } => {
                let type_value = ((value_arity as u64) << 32) | 1_u64;
                Felt::new(type_value)
            },
            StorageSlotType::Array { depth, value_arity } => {
                let type_value = ((value_arity as u64) << 32) | (depth as u64);
                Felt::new(type_value)
            },
        }
    }
}

impl From<&StorageSlotType> for Felt {
    fn from(value: &StorageSlotType) -> Self {
        Self::from(*value)
    }
}
