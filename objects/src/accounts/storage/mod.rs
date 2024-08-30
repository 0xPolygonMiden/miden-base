use alloc::{collections::BTreeMap, string::ToString, vec::Vec};
use vm_core::EMPTY_WORD;

use super::{
    AccountError, AccountStorageDelta, ByteReader, ByteWriter, Deserializable,
    DeserializationError, Digest, Felt, Hasher, Serializable, Word,
};
use crate::crypto::merkle::LeafIndex;

mod slot;
pub use slot::{StorageSlot, StorageSlotType};

mod map;
pub use map::StorageMap;

// CONSTANTS
// ================================================================================================

/// Total number of storage slots.
pub const NUM_STORAGE_SLOTS: usize = 256;

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
    slots: Vec<StorageSlot>,
    commitment: Digest,
}

impl AccountStorage {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new instance of account storage initialized with the provided items.
    pub fn new(slots: Vec<StorageSlot>) -> Result<AccountStorage, AccountError> {
        let len = slots.len();

        if len > NUM_STORAGE_SLOTS {
            return Err(AccountError::StorageTooManySlots(len as u64));
        }

        Ok(Self {
            commitment: build_slots_commitment(&slots),
            slots,
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a reference to the storage slots.
    pub fn slots(&self) -> &Vec<StorageSlot> {
        &self.slots
    }

    /// Returns a commitment to this storage.
    pub fn commitment(&self) -> Digest {
        self.commitment
    }

    /// Returns an item from the storage at the specified index.
    ///
    /// If the item is not present in the storage, [crate::EMPTY_WORD] is returned.
    pub fn get_item(&self, index: u8) -> Digest {
        if self.slots.len() < index as usize {
            return Digest::default();
        };

        Digest::from(self.slots[index as usize].get_value_as_word())
    }

    /// Returns a map item from the storage at the specified index.
    ///
    /// If the item is not present in the storage, [crate::EMPTY_WORD] is returned.
    pub fn get_map_item(&self, index: u8, key: Word) -> Result<Word, AccountError> {
        if self.slots.len() < index as usize {
            return Ok(EMPTY_WORD);
        };

        let storage_slot = self.slots[index as usize].clone();

        match storage_slot {
            StorageSlot::Map(map) => Ok(map.get_value(&Digest::from(key))),
            _ => return Err(AccountError::StorageMapNotFound(index)),
        }
    }

    // DATA MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Applies the provided delta to this account storage.
    ///
    /// # Errors
    /// Returns an error if the updates violate storage layout constraints.
    pub(super) fn apply_delta(&mut self, delta: &AccountStorageDelta) -> Result<(), AccountError> {
        // --- update storage maps --------------------------------------------

        for (&slot_idx, map_delta) in delta.maps().iter() {
            let storage_map =
                self.maps.get_mut(&slot_idx).ok_or(AccountError::StorageMapNotFound(slot_idx))?;

            let new_root = storage_map.apply_delta(map_delta);

            let index = LeafIndex::new(slot_idx.into()).expect("index is u8 - index within range");
            self.slots.insert(index, new_root.into());
        }

        // --- update storage slots -------------------------------------------

        for (&slot_idx, &slot_value) in delta.slots().iter() {
            self.set_item(slot_idx, slot_value)?;
        }

        Ok(())
    }

    /// Updates the value of the storage slot at the specified index.
    ///
    /// This method should be used only to update simple value slots. For updating values
    /// in storage maps, please see [AccountStorage::set_map_item()].
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
            slot_type => Err(AccountError::StorageSlotMapOrArrayNotAllowed(index, slot_type))?,
        }

        // update the slot and return
        let index = LeafIndex::new(index.into()).expect("index is u8 - index within range");
        let slot_value = self.slots.insert(index, value);
        Ok(slot_value)
    }

    /// Updates the value of a key-value pair of a storage map at the specified index.
    ///
    /// This method should be used only to update storage maps. For updating values
    /// in storage slots, please see [AccountStorage::set_item()].
    ///
    /// # Errors
    /// Returns an error if:
    /// - The index specifies a reserved storage slot.
    /// - The index is not a map slot.
    /// - The update tries to set a slot of type value or array.
    /// - The update has a value arity different from 0.
    pub fn set_map_item(
        &mut self,
        index: u8,
        key: Word,
        value: Word,
    ) -> Result<(Word, Word), AccountError> {
        // layout commitment slot cannot be updated
        if index == Self::SLOT_LAYOUT_COMMITMENT_INDEX {
            return Err(AccountError::StorageSlotIsReserved(index));
        }

        // only map slots of basic arity can currently be updated
        match self.layout[index as usize] {
            StorageSlotType::Map { value_arity } => {
                if value_arity > 0 {
                    return Err(AccountError::StorageSlotInvalidValueArity {
                        slot: index,
                        expected: 0,
                        actual: value_arity,
                    });
                }
            },
            slot_type => Err(AccountError::MapsUpdateToNonMapsSlot(index, slot_type))?,
        }

        // get the correct map
        let storage_map =
            self.maps.get_mut(&index).ok_or(AccountError::StorageMapNotFound(index))?;

        // get old map root to return
        let old_map_root = storage_map.root();

        // update the key-value pair in the map
        let old_value = storage_map.insert(key.into(), value);

        // update the root of the storage map in the corresponding storage slot
        let index = LeafIndex::new(index.into()).expect("index is u8 - index within range");
        self.slots.insert(index, storage_map.root().into());

        Ok((old_map_root.into(), old_value))
    }
}

// HELPER FUNCTIONS
// ------------------------------------------------------------------------------------------------

/// Converts given slots into field elements
fn slots_as_elements(slots: &[StorageSlot]) -> Vec<Felt> {
    let mut elements = Vec::new();

    slots.iter().map(|slot| {
        elements.extend_from_slice(&slot.as_elements());
    });

    elements
}

/// Computes the commitment to the given slots
fn build_slots_commitment(slots: &[StorageSlot]) -> Digest {
    let elements = slots_as_elements(slots);
    Hasher::hash_elements(&elements)
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
