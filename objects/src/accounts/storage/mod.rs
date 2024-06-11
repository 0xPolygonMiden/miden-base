use alloc::{collections::BTreeMap, string::ToString, vec::Vec};

use super::{
    AccountError, AccountStorageDelta, ByteReader, ByteWriter, Deserializable,
    DeserializationError, Digest, Felt, Hasher, Serializable, Word,
};
use crate::crypto::merkle::{LeafIndex, NodeIndex, SimpleSmt};

mod slot;
pub use slot::StorageSlotType;

mod map;
pub use map::StorageMap;

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

impl SlotItem {
    /// Returns a new [SlotItem] with the [StorageSlotType::Value] type.
    pub fn new_value(index: u8, arity: u8, value: Word) -> Self {
        Self {
            index,
            slot: StorageSlot {
                slot_type: StorageSlotType::Value { value_arity: arity },
                value,
            },
        }
    }

    /// Returns a new [SlotItem] with the [StorageSlotType::Map] type.
    pub fn new_map(index: u8, arity: u8, root: Word) -> Self {
        Self {
            index,
            slot: StorageSlot {
                slot_type: StorageSlotType::Map { value_arity: arity },
                value: root,
            },
        }
    }

    /// Returns a new [SlotItem] with the [StorageSlotType::Array] type.
    ///
    /// The max size of the array is set to 2^log_n and the value arity for the slot is set to 0.
    pub fn new_array(index: u8, arity: u8, log_n: u8, root: Word) -> Self {
        Self {
            index,
            slot: StorageSlot {
                slot_type: StorageSlotType::Array { depth: log_n, value_arity: arity },
                value: root,
            },
        }
    }
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
///
/// Optionally, a user can make use of storage maps. Storage maps are represented by a SMT and
/// they can hold more data as there is in plain usage of the storage slots. The root of the SMT
/// consumes one storage slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountStorage {
    slots: SimpleSmt<STORAGE_TREE_DEPTH>,
    layout: Vec<StorageSlotType>,
    maps: BTreeMap<u8, StorageMap>,
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
    pub fn new(
        items: Vec<SlotItem>,
        maps: BTreeMap<u8, StorageMap>,
    ) -> Result<AccountStorage, AccountError> {
        // Empty layout
        let mut layout = vec![StorageSlotType::default(); AccountStorage::NUM_STORAGE_SLOTS];
        layout[usize::from(AccountStorage::SLOT_LAYOUT_COMMITMENT_INDEX)] =
            StorageSlotType::Value { value_arity: 64 };

        // The following loop will:
        //
        // - Validate the slot and check it doesn't assign a value to a reserved slot.
        // - Extract the slot value.
        // - Check that every map index has a corresponding map in `maps`.
        // - Count the number of maps to validate `maps`.
        //
        // It won't detect duplicates, that is later done by the `SimpleSmt` instantiation.
        //
        let mut entries = Vec::with_capacity(AccountStorage::NUM_STORAGE_SLOTS);
        let mut num_maps = 0;
        for item in items {
            if item.index == AccountStorage::SLOT_LAYOUT_COMMITMENT_INDEX {
                return Err(AccountError::StorageSlotIsReserved(item.index));
            }

            if matches!(item.slot.slot_type, StorageSlotType::Map { .. }) {
                // check that for every map index there is a map in maps
                if !maps.contains_key(&item.index) {
                    return Err(AccountError::StorageMapNotFound(item.index));
                }
                num_maps += 1;
            }

            layout[usize::from(item.index)] = item.slot.slot_type;
            entries.push((item.index.into(), item.slot.value))
        }

        // add layout commitment entry
        entries.push((
            AccountStorage::SLOT_LAYOUT_COMMITMENT_INDEX.into(),
            *layout_commitment(&layout),
        ));

        // construct storage slots smt and populate the types vector.
        let slots = SimpleSmt::<STORAGE_TREE_DEPTH>::with_leaves(entries)
            .map_err(AccountError::DuplicateStorageItems)?;

        if maps.len() > num_maps {
            return Err(AccountError::StorageMapTooManyMaps {
                expected: num_maps,
                actual: maps.len(),
            });
        }

        Ok(Self { slots, layout, maps })
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
        let item_index = NodeIndex::new(Self::STORAGE_TREE_DEPTH, index.into())
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
        layout_commitment(&self.layout)
    }

    /// Returns the storage maps for this storage.
    pub fn maps(&self) -> &BTreeMap<u8, StorageMap> {
        &self.maps
    }

    // DATA MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Applies the provided delta to this account storage.
    ///
    /// This method assumes that the delta has been validated by the calling method and so, no
    /// additional validation of delta is performed.
    ///
    /// Returns an error if:
    /// - The delta implies an update to a reserved account slot.
    /// - The updates violate storage layout constraints.
    /// - The updated value has an arity different from 0.
    pub(super) fn apply_delta(&mut self, delta: &AccountStorageDelta) -> Result<(), AccountError> {
        for &slot_idx in delta.cleared_items.iter() {
            self.set_item(slot_idx, Word::default())?;
        }

        for &(slot_idx, slot_value) in delta.updated_items.iter() {
            self.set_item(slot_idx, slot_value)?;
        }

        for &(slot_idx, ref map_delta) in delta.updated_maps.iter() {
            // layout commitment slot cannot be updated
            if slot_idx == Self::SLOT_LAYOUT_COMMITMENT_INDEX {
                return Err(AccountError::StorageSlotIsReserved(slot_idx));
            }

            // pick the right storage map and apply delta
            let storage_map =
                self.maps.get_mut(&slot_idx).ok_or(AccountError::StorageMapNotFound(slot_idx))?;
            storage_map.apply_delta(map_delta)?;
        }

        Ok(())
    }

    /// Sets an item from the storage at the specified index.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The index specifies a reserved storage slot.
    /// - The update tries to set a slot of type array.
    /// - The update has a value arity different from 0.
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
            StorageSlotType::Map { value_arity } => {
                if value_arity > 0 {
                    return Err(AccountError::StorageSlotInvalidValueArity {
                        slot: index,
                        expected: 0,
                        actual: value_arity,
                    });
                }
            },
            slot_type => Err(AccountError::StorageSlotArrayNotAllowed(index, slot_type))?,
        }

        // update the slot and return
        let index = LeafIndex::new(index.into()).expect("index is u8 - index within range");
        let slot_value = self.slots.insert(index, value);
        Ok(slot_value)
    }
}

// UTILITIES
// ------------------------------------------------------------------------------------------------

/// Computes the commitment to the given layout
fn layout_commitment(layout: &[StorageSlotType]) -> Digest {
    Hasher::hash_elements(&layout.iter().map(Felt::from).collect::<Vec<_>>())
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountStorage {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        // don't serialize last slot as it is a constant.
        // complex types are all types different from StorageSlotType::Value { value_arity: 0 }
        let complex_types = self.layout[..usize::from(AccountStorage::SLOT_LAYOUT_COMMITMENT_INDEX)]
            .iter()
            .enumerate()
            // don't serialize default types, these are implied.
            .filter(|(_, slot_type)| !slot_type.is_default())
            .map(|(index, slot_type)| (u8::try_from(index).expect("Number of slot types is limited to u8"), slot_type))
            .collect::<Vec<_>>();

        complex_types.write_into(target);

        let filled_slots = self
            .slots
            .leaves()
            // don't serialize the default values, these are implied.
            .filter(|(index, &value)| {
                let slot_type = self.layout
                    [usize::try_from(*index).expect("Number of slot types is limited to u8")];
                value != slot_type.default_word()
            })
            .map(|(index, value)| (u8::try_from(index).expect("Number of slot types is limited to u8"), value))
            // don't serialized the layout commitment, it can be recomputed
            .filter(|(index, _)| *index != AccountStorage::SLOT_LAYOUT_COMMITMENT_INDEX)
            .collect::<Vec<_>>();

        filled_slots.write_into(target);

        // serialize the storage maps
        self.maps.write_into(target);
    }
}

impl Deserializable for AccountStorage {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        // read the non-default layout types
        let complex_types = <Vec<(u8, StorageSlotType)>>::read_from(source)?;
        let mut complex_types = BTreeMap::from_iter(complex_types);

        // read the non-default entries
        let filled_slots = <Vec<(u8, Word)>>::read_from(source)?;
        let mut items: Vec<SlotItem> = Vec::new();
        for (index, value) in filled_slots {
            let slot_type = complex_types.remove(&index).unwrap_or_default();
            items.push(SlotItem {
                index,
                slot: StorageSlot { slot_type, value },
            });
        }
        // read the storage maps
        let maps = <BTreeMap<u8, StorageMap>>::read_from(source)?;

        Self::new(items, maps).map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeMap, vec::Vec};

    use miden_crypto::hash::rpo::RpoDigest;

    use super::{AccountStorage, Deserializable, Felt, Serializable, SlotItem, StorageMap, Word};
    use crate::{ONE, ZERO};

    #[test]
    fn account_storage_serialization() {
        // empty storage
        let storage = AccountStorage::new(Vec::new(), BTreeMap::new()).unwrap();
        let bytes = storage.to_bytes();
        assert_eq!(storage, AccountStorage::read_from_bytes(&bytes).unwrap());

        // storage with values for default types
        let storage = AccountStorage::new(
            vec![
                SlotItem::new_value(0, 0, [ONE, ONE, ONE, ONE]),
                SlotItem::new_value(2, 0, [ONE, ONE, ONE, ZERO]),
            ],
            BTreeMap::new(),
        )
        .unwrap();
        let bytes = storage.to_bytes();
        assert_eq!(storage, AccountStorage::read_from_bytes(&bytes).unwrap());

        // storage with values for complex types
        let storage_map_leaves_2: [(RpoDigest, Word); 2] = [
            (
                RpoDigest::new([Felt::new(101), Felt::new(102), Felt::new(103), Felt::new(104)]),
                [Felt::new(1_u64), Felt::new(2_u64), Felt::new(3_u64), Felt::new(4_u64)],
            ),
            (
                RpoDigest::new([Felt::new(105), Felt::new(106), Felt::new(107), Felt::new(108)]),
                [Felt::new(5_u64), Felt::new(6_u64), Felt::new(7_u64), Felt::new(8_u64)],
            ),
        ];
        let storage_map = StorageMap::with_entries(storage_map_leaves_2).unwrap();
        let mut maps = BTreeMap::new();
        maps.insert(2, storage_map.clone());
        let storage = AccountStorage::new(
            vec![
                SlotItem::new_value(0, 1, [ONE, ONE, ONE, ONE]),
                SlotItem::new_value(1, 0, [ONE, ONE, ONE, ZERO]),
                SlotItem::new_map(2, 0, storage_map.root().into()),
                SlotItem::new_array(3, 3, 4, [ONE, ZERO, ZERO, ZERO]),
            ],
            maps,
        )
        .unwrap();

        let bytes = storage.to_bytes();
        assert_eq!(storage, AccountStorage::read_from_bytes(&bytes).unwrap());
    }
}
