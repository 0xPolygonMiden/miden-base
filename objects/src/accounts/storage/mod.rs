use alloc::{collections::BTreeMap, vec::Vec};
use vm_core::{EMPTY_WORD, ONE, ZERO};

use super::{
    AccountError, AccountStorageDelta, ByteReader, ByteWriter, Deserializable,
    DeserializationError, Digest, Felt, Hasher, Serializable, Word,
};

mod slot;
use slot::StorageSlot;

mod map;
pub use map::StorageMap;

// CONSTANTS
// ================================================================================================

// ACCOUNT STORAGE
// ================================================================================================

/// Account storage consists of 256 index-addressable storage slots.
///
/// Each slot has a type which defines the size and the structure of the slot. Currently, the
/// following types are supported:
/// - Value: a Word.
/// - Map: a key-value map where keys are words and values contain up to 256 words.
///
/// A user can make use of storage maps. Storage maps are represented by a SMT and
/// they can hold more data as there is in plain usage of the storage slots. The root of the SMT
/// consumes one storage slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountStorage {
    slots: Vec<StorageSlot>,
    commitment: Digest,
}

impl AccountStorage {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// Total number of storage slots.
    pub const NUM_STORAGE_SLOTS: usize = 256;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of account storage initialized with the provided slots.
    pub fn new(slots: &[StorageSlot]) -> Self {
        Self {
            slots: slots.to_vec(),
            commitment: build_slots_commitment(&slots),
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to this storage.
    pub fn commitment(&self) -> Digest {
        self.commitment
    }

    /// Returns an item from the storage at the specified index.
    ///
    /// If the item is not present in the storage, [crate::EMPTY_WORD] is returned.
    pub fn get_item(&self, index: u8) -> Digest {
        match self.slots.get(index as usize) {
            Some(storage_slot) => match storage_slot {
                StorageSlot::Value(word) => word.into(),
                StorageSlot::Map(map) => map.root(),
            },
            None => EMPTY_WORD.into(),
        }
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
        // --- update storage maps --------------------------------------------

        for &(slot_idx, ref map_delta) in delta.updated_maps.iter() {
            let storage_map =
                self.maps.get_mut(&slot_idx).ok_or(AccountError::StorageMapNotFound(slot_idx))?;

            let new_root = storage_map.apply_delta(map_delta)?;

            let index = LeafIndex::new(slot_idx.into()).expect("index is u8 - index within range");
            self.slots.insert(index, new_root.into());
        }

        // --- update storage slots -------------------------------------------

        for &slot_idx in delta.cleared_items.iter() {
            self.set_item(slot_idx, Word::default())?;
        }

        for &(slot_idx, slot_value) in delta.updated_items.iter() {
            self.set_item(slot_idx, slot_value)?;
        }

        Ok(())
    }

    /// Updates the value of the storage slot at the specified index.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The index specifies a reserved storage slot.
    /// - The update tries to set a slot of type array.
    /// - The update has a value arity different from 0.
    pub fn set_item(&mut self, index: u8, storage_slot: StorageSlot) {
        self.slots[index as usize] = storage_slot;
    }
}

// HELPER FUNCTIONS
// ------------------------------------------------------------------------------------------------

/// Convers given slots into field elements
fn slots_as_elements(slots: &[StorageSlot]) -> Vec<Felt> {
    slots
        .iter()
        .flat_map(|storage_slot| {
            let mut elements: Vec<Felt> = Vec::with_capacity(8);
            match storage_slot {
                StorageSlot::Value(word) => {
                    for element in word {
                        elements.push(*element)
                    }
                    for _ in 0..4 {
                        elements.push(ZERO)
                    }
                },
                StorageSlot::Map(map) => {
                    let smt_root = map.root();

                    for element in smt_root.as_elements() {
                        elements.push(*element)
                    }

                    elements.push(ONE);

                    for _ in 0..3 {
                        elements.push(ZERO)
                    }
                },
            }
            elements
        })
        .collect()
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
        target.write(self.slots.clone());
        target.write(self.commitment);
    }
}

impl Deserializable for AccountStorage {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let slots = source.read();
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
