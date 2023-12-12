use super::Felt;

/// An object that represents the type of a storage slot.
///
/// The type of the value in a storage entry is described by the [ValueType] object.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum StorageSlotType {
    /// Represents a slot that contains a value with the specified arity.
    Value { value_arity: u8 },
    /// Represents a slot that contains a commitment to a map with values with the specified arity.
    Map { value_arity: u8 },
    /// Represents a slot that contains a commitment to an array with capacity 2^depth with values
    /// with the specified arity.
    Array { depth: u8, value_arity: u8 },
}

impl From<&StorageSlotType> for Felt {
    fn from(slot_type: &StorageSlotType) -> Self {
        match slot_type {
            StorageSlotType::Value { value_arity } => {
                let type_value = (*value_arity as u64) << 32;
                Felt::from(type_value)
            }
            StorageSlotType::Map { value_arity } => {
                let type_value = ((*value_arity as u64) << 32) | 1_u64;
                Felt::from(type_value)
            }
            StorageSlotType::Array { depth, value_arity } => {
                let type_value = ((*value_arity as u64) << 32) | (*depth as u64);
                Felt::from(type_value)
            }
        }
    }
}
