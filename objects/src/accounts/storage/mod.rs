use alloc::{string::ToString, vec::Vec};

use vm_core::EMPTY_WORD;

use super::{
    AccountError, AccountStorageDelta, ByteReader, ByteWriter, Deserializable,
    DeserializationError, Digest, Felt, Hasher, Serializable, Word,
};

mod slot;
pub use slot::{StorageSlot, StorageSlotType};

mod map;
pub use map::StorageMap;

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
}

impl AccountStorage {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// Total number of storage slots.
    pub const NUM_STORAGE_SLOTS: usize = 256;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of account storage initialized with the provided items.
    pub fn new(slots: Vec<StorageSlot>) -> Result<AccountStorage, AccountError> {
        let len = slots.len();

        if len > Self::NUM_STORAGE_SLOTS {
            return Err(AccountError::StorageTooManySlots(len as u64));
        }

        Ok(Self { slots })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a reference to the storage slots.
    pub fn slots(&self) -> &Vec<StorageSlot> {
        &self.slots
    }

    /// Returns a commitment to this storage.
    pub fn commitment(&self) -> Digest {
        build_slots_commitment(&self.slots)
    }

    /// Returns the number of storage slots contained in this storage.
    pub fn num_slots(&self) -> usize {
        self.slots.len()
    }

    /// Converts storage slots of this [AccountStorage] into a vector of field elements.
    ///
    /// This is done by first converting each procedure into exactly 8 elements as follows:
    /// ```text
    /// [STORAGE_SLOT_VALUE, storage_slot_type, 0, 0, 0]
    /// ```
    /// And then concatenating the resulting elements into a single vector.
    pub fn slots_as_elements(&self) -> Vec<Felt> {
        slots_as_elements(self.slots())
    }

    /// Returns an item from the storage at the specified index.
    ///
    /// If the item is not present in the storage, [crate::EMPTY_WORD] is returned.
    pub fn get_item(&self, index: u8) -> Digest {
        Digest::from(
            self.slots
                .get(index as usize)
                .map(|slot| slot.get_value_as_word())
                .unwrap_or(EMPTY_WORD),
        )
    }

    /// Returns a map item from the storage at the specified index.
    ///
    /// If the item is not present in the storage, [crate::EMPTY_WORD] is returned.
    pub fn get_map_item(&self, index: u8, key: Word) -> Result<Word, AccountError> {
        match self.slots.get(index as usize) {
            Some(StorageSlot::Map(map)) => Ok(map.get_value(&Digest::from(key))),
            Some(_) => Err(AccountError::StorageSlotNotMap(index)),
            None => Ok(EMPTY_WORD),
        }
    }

    // DATA MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Applies the provided delta to this account storage.
    ///
    /// # Errors
    /// Returns an error if the updates violate storage constraints.
    pub(super) fn apply_delta(&mut self, delta: &AccountStorageDelta) -> Result<(), AccountError> {
        // --- update storage maps --------------------------------------------

        for (&idx, map) in delta.maps().iter() {
            let storage_slot =
                self.slots.get_mut(idx as usize).ok_or(AccountError::StorageMapNotFound(idx))?;

            let storage_map = match storage_slot {
                StorageSlot::Map(map) => map,
                _ => return Err(AccountError::StorageMapNotFound(idx)),
            };

            storage_map.apply_delta(map);
        }

        // --- update storage slots -------------------------------------------

        for (&idx, &value) in delta.values().iter() {
            self.set_item(idx, value)?;
        }

        Ok(())
    }

    /// Updates the value of the storage slot at the specified index.
    ///
    /// This method should be used only to update simple value slots. For updating values
    /// in storage maps, please see [AccountStorage::set_map_item()].
    pub fn set_item(&mut self, index: u8, value: Word) -> Result<Word, AccountError> {
        let len = self.slots.len();

        if index as usize >= len {
            return Err(AccountError::StorageIndexOutOfBounds(index));
        }

        // update the slot and return
        let old_value = self.slots[index as usize].clone();

        self.slots[index as usize] = StorageSlot::Value(value);

        Ok(old_value.get_value_as_word())
    }

    /// Updates the value of a key-value pair of a storage map at the specified index.
    ///
    /// This method should be used only to update storage maps. For updating values
    /// in storage slots, please see [AccountStorage::set_item()].
    ///
    /// # Errors
    /// Returns an error if:
    /// - The index is not a map slot.
    pub fn set_map_item(
        &mut self,
        index: u8,
        key: Word,
        value: Word,
    ) -> Result<(Word, Word), AccountError> {
        let len = self.slots.len() - 1;

        if len < index as usize {
            return Err(AccountError::StorageIndexOutOfBounds(index));
        }

        let storage_slot = self.slots[index as usize].clone();

        let mut storage_map = match storage_slot {
            StorageSlot::Map(map) => map,
            _ => return Err(AccountError::StorageMapNotFound(index)),
        };

        // get old map root to return
        let old_root = storage_map.root();

        // update the key-value pair in the map
        let old_value = storage_map.insert(key.into(), value);

        Ok((old_root.into(), old_value))
    }
}

// HELPER FUNCTIONS
// ------------------------------------------------------------------------------------------------

/// Converts given slots into field elements
fn slots_as_elements(slots: &[StorageSlot]) -> Vec<Felt> {
    slots.iter().flat_map(|slot| slot.as_elements()).collect()
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
        target.write_u16(self.slots().len() as u16);
        target.write_many(self.slots());
    }
}

impl Deserializable for AccountStorage {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let num_slots = source.read_u16()? as usize;
        let slots = source.read_many::<StorageSlot>(num_slots)?;

        Self::new(slots).map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::{AccountStorage, Deserializable, Serializable, StorageMap, Word};
    use crate::accounts::StorageSlot;

    #[test]
    fn test_serde_account_storage() {
        // empty storage
        let storage = AccountStorage::new(vec![]).unwrap();
        let bytes = storage.to_bytes();
        assert_eq!(storage, AccountStorage::read_from_bytes(&bytes).unwrap());

        // storage with values for default types
        let storage = AccountStorage::new(vec![
            StorageSlot::Value(Word::default()),
            StorageSlot::Map(StorageMap::default()),
        ])
        .unwrap();
        let bytes = storage.to_bytes();
        assert_eq!(storage, AccountStorage::read_from_bytes(&bytes).unwrap());
    }
}
