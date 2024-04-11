use alloc::{collections::BTreeMap, string::ToString, vec::Vec};

use super::{
    AccountError, AccountStorageDelta, ByteReader, ByteWriter, Deserializable,
    DeserializationError, Digest, Felt, Hasher, Serializable, Word,
};
use crate::crypto::merkle::{LeafIndex, NodeIndex, SimpleSmt};

mod slot;
pub use slot::StorageSlotType;

// CONSTANTS
// ================================================================================================

/// Depth of the storage tree.
pub const STORAGE_TREE_DEPTH: u8 = 8;

// TYPE ALIASES
// ================================================================================================

/// Represents a single storage slot item.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct SlotItem {
    /// The index this item will occupy in the [AccountStorage] tree.
    pub index: u8,

    /// The type and value of the item.
    pub slot: StorageSlot,
}

/// Represents a single storage slot entry.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct StorageSlot {
    /// The type of the value
    pub slot_type: StorageSlotType,

    /// The value itself.
    ///
    /// The value can be a raw value or a commitment to the underlying data structure.
    pub value: Word,
}

impl StorageSlot {
    /// Returns a new [StorageSlot] with the provided value.
    ///
    /// The value arity for the slot is set to 0.
    pub fn new_value(value: Word) -> Self {
        Self {
            slot_type: StorageSlotType::Value { value_arity: 0 },
            value,
        }
    }

    /// Returns a new [StorageSlot] with a map defined by the provided root.
    ///
    /// The value arity for the slot is set to 0.
    pub fn new_map(root: Word) -> Self {
        Self {
            slot_type: StorageSlotType::Map { value_arity: 0 },
            value: root,
        }
    }

    /// Returns a new [StorageSlot] with an array defined by the provided root and the number of
    /// elements.
    ///
    /// The max size of the array is set to 2^log_n and the value arity for the slot is set to 0.
    pub fn new_array(root: Word, log_n: u8) -> Self {
        Self {
            slot_type: StorageSlotType::Array { depth: log_n, value_arity: 0 },
            value: root,
        }
    }
}

// ACCOUNT STORAGE
// ================================================================================================

/// Account storage consists of 256 index-addressable storage slots.
///
/// Each slot has a type which defines the size and the structure of the slot. Currently, the
/// following types are supported:
/// - Scalar: a sequence of up to 256 words.
/// - Array: a sparse array of up to 2^n values where n > 1 and n <= 64 and each value contains up
///   to 256 words.
/// - Map: a key-value map where keys are words and values contain up to 256 words.
///
/// Storage slots are stored in a simple Sparse Merkle Tree of depth 8. Slot 255 is always reserved
/// and contains information about slot types of all other slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountStorage {
    slots: SimpleSmt<STORAGE_TREE_DEPTH>,
    layout: Vec<StorageSlotType>,
}

impl AccountStorage {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// Depth of the storage tree.
    pub const STORAGE_TREE_DEPTH: u8 = STORAGE_TREE_DEPTH;

    /// Total number of storage slots.
    pub const NUM_STORAGE_SLOTS: usize = 256;

    /// The storage slot at which the layout commitment is stored.
    pub const SLOT_LAYOUT_COMMITMENT_INDEX: u8 = 255;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new instance of account storage initialized with the provided items.
    pub fn new(items: Vec<SlotItem>) -> Result<AccountStorage, AccountError> {
        // initialize storage layout
        let mut layout = vec![StorageSlotType::default(); Self::NUM_STORAGE_SLOTS];

        // set the slot type for the layout commitment
        layout[Self::SLOT_LAYOUT_COMMITMENT_INDEX as usize] =
            StorageSlotType::Value { value_arity: 64 };

        // process entries to extract type data
        let mut entries = items
            .into_iter()
            .map(|item| {
                if item.index == Self::SLOT_LAYOUT_COMMITMENT_INDEX {
                    return Err(AccountError::StorageSlotIsReserved(item.index));
                }

                layout[item.index as usize] = item.slot.slot_type;
                Ok((item.index as u64, item.slot.value))
            })
            .collect::<Result<Vec<_>, AccountError>>()?;

        // add layout commitment entry
        entries.push((
            Self::SLOT_LAYOUT_COMMITMENT_INDEX as u64,
            *Hasher::hash_elements(&layout.iter().map(Felt::from).collect::<Vec<_>>()),
        ));

        // construct storage slots smt and populate the types vector.
        let slots = SimpleSmt::<STORAGE_TREE_DEPTH>::with_leaves(entries)
            .map_err(AccountError::DuplicateStorageItems)?;

        Ok(Self { slots, layout })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to this storage.
    pub fn root(&self) -> Digest {
        self.slots.root()
    }

    /// Returns an item from the storage at the specified index.
    ///
    /// If the item is not present in the storage, [ZERO; 4] is returned.
    pub fn get_item(&self, index: u8) -> Digest {
        let item_index = NodeIndex::new(Self::STORAGE_TREE_DEPTH, index as u64)
            .expect("index is u8 - index within range");
        self.slots.get_node(item_index).expect("index is u8 - index within range")
    }

    /// Returns a reference to the Sparse Merkle Tree that backs the storage slots.
    pub fn slots(&self) -> &SimpleSmt<STORAGE_TREE_DEPTH> {
        &self.slots
    }

    /// Returns layout info for this storage.
    pub fn layout(&self) -> &[StorageSlotType] {
        &self.layout
    }

    /// Returns a commitment to the storage layout.
    pub fn layout_commitment(&self) -> Digest {
        Hasher::hash_elements(&self.layout.iter().map(Felt::from).collect::<Vec<_>>())
    }

    // DATA MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Applies the provided delta to this account storage.
    ///
    /// This method assumes that the delta has been validated by the calling method and so, no
    /// additional validation of delta is performed.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The delta implies an update to a reserved account slot.
    /// - The updates violate storage layout constraints.
    pub(super) fn apply_delta(&mut self, delta: &AccountStorageDelta) -> Result<(), AccountError> {
        for &slot_idx in delta.cleared_items.iter() {
            self.set_item(slot_idx, Word::default())?;
        }

        for &(slot_idx, slot_value) in delta.updated_items.iter() {
            self.set_item(slot_idx, slot_value)?;
        }

        Ok(())
    }

    /// Sets an item from the storage at the specified index.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The index specifies a reserved storage slot.
    /// - The update violates storage layout constraints.
    pub fn set_item(&mut self, index: u8, value: Word) -> Result<Word, AccountError> {
        // layout commitment slot cannot be updated
        if index == Self::SLOT_LAYOUT_COMMITMENT_INDEX {
            return Err(AccountError::StorageSlotIsReserved(index));
        }

        // only value slots of basic arity can currently be updated
        match self.layout[index as usize] {
            StorageSlotType::Value { value_arity } => {
                if value_arity > 0 {
                    return Err(AccountError::StorageSlotInvalidValueArity {
                        slot: index,
                        expected: 0,
                        actual: value_arity,
                    });
                }
            },
            slot_type => Err(AccountError::StorageSlotNotValueSlot(index, slot_type))?,
        }

        // update the slot and return
        let index = LeafIndex::new(index as u64).expect("index is u8 - index within range");
        let slot_value = self.slots.insert(index, value);
        Ok(slot_value)
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountStorage {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        // serialize layout info; we don't serialize default type info as we'll assume that any
        // slot type that wasn't serialized was a default slot type. also we skip the last slot
        // type as it is a constant.
        let complex_types = self.layout[..255]
            .iter()
            .enumerate()
            .filter(|(_, slot_type)| !slot_type.is_default())
            .collect::<Vec<_>>();

        target.write_u8(complex_types.len() as u8);
        for (idx, slot_type) in complex_types {
            target.write_u8(idx as u8);
            target.write_u16(slot_type.into());
        }

        // serialize slot values; we serialize only non-empty values and also skip slot 255 as info
        // for this slot was already serialized as a part of serializing slot type info above
        let filled_slots = self
            .slots
            .leaves()
            .filter(|(idx, &value)| {
                // TODO: consider checking empty values for complex types as well
                value != SimpleSmt::<STORAGE_TREE_DEPTH>::EMPTY_VALUE
                    && *idx as u8 != AccountStorage::SLOT_LAYOUT_COMMITMENT_INDEX
            })
            .collect::<Vec<_>>();

        target.write_u8(filled_slots.len() as u8);
        for (idx, &value) in filled_slots {
            target.write_u8(idx as u8);
            target.write(value);
        }
    }
}

impl Deserializable for AccountStorage {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        // read complex types
        let mut complex_types = BTreeMap::new();
        let num_complex_types = source.read_u8()?;
        for _ in 0..num_complex_types {
            let idx = source.read_u8()?;
            let slot_type: StorageSlotType =
                source.read_u16()?.try_into().map_err(DeserializationError::InvalidValue)?;
            complex_types.insert(idx, slot_type);
        }

        // read filled slots and build a vector of slot items
        let mut items: Vec<SlotItem> = Vec::new();
        let num_filled_slots = source.read_u8()?;
        for _ in 0..num_filled_slots {
            let index = source.read_u8()?;
            let value: Word = source.read()?;
            let slot_type = complex_types.remove(&index).unwrap_or_default();
            items.push(SlotItem {
                index,
                slot: StorageSlot { slot_type, value },
            });
        }

        Self::new(items).map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::{
        AccountStorage, Deserializable, Serializable, SlotItem, StorageSlot, StorageSlotType,
    };
    use crate::{ONE, ZERO};

    #[test]
    fn account_storage_serialization() {
        // empty storage
        let storage = AccountStorage::new(Vec::new()).unwrap();
        let bytes = storage.to_bytes();
        assert_eq!(storage, AccountStorage::read_from_bytes(&bytes).unwrap());

        // storage with values for default types
        let storage = AccountStorage::new(vec![
            SlotItem {
                index: 0,
                slot: StorageSlot {
                    slot_type: StorageSlotType::default(),
                    value: [ONE, ONE, ONE, ONE],
                },
            },
            SlotItem {
                index: 2,
                slot: StorageSlot {
                    slot_type: StorageSlotType::default(),
                    value: [ONE, ONE, ONE, ZERO],
                },
            },
        ])
        .unwrap();
        let bytes = storage.to_bytes();
        assert_eq!(storage, AccountStorage::read_from_bytes(&bytes).unwrap());

        // storage with a mix of types
        let storage = AccountStorage::new(vec![
            SlotItem {
                index: 0,
                slot: StorageSlot {
                    slot_type: StorageSlotType::Value { value_arity: 1 },
                    value: [ONE, ONE, ONE, ONE],
                },
            },
            SlotItem {
                index: 1,
                slot: StorageSlot::new_value([ONE, ONE, ONE, ZERO]),
            },
            SlotItem {
                index: 2,
                slot: StorageSlot {
                    slot_type: StorageSlotType::Map { value_arity: 2 },
                    value: [ONE, ONE, ZERO, ZERO],
                },
            },
            SlotItem {
                index: 3,
                slot: StorageSlot {
                    slot_type: StorageSlotType::Array { depth: 4, value_arity: 3 },
                    value: [ONE, ZERO, ZERO, ZERO],
                },
            },
        ])
        .unwrap();
        let bytes = storage.to_bytes();
        assert_eq!(storage, AccountStorage::read_from_bytes(&bytes).unwrap());
    }
}
