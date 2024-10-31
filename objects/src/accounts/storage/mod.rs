use alloc::{string::ToString, vec::Vec};

use super::{
    AccountError, AccountStorageDelta, ByteReader, ByteWriter, Deserializable,
    DeserializationError, Digest, Felt, Hasher, Serializable, Word,
};
use crate::accounts::AccountComponent;

mod slot;
pub use slot::{StorageSlot, StorageSlotType};

mod map;
pub use map::StorageMap;

mod header;
pub use header::AccountStorageHeader;

// ACCOUNT STORAGE
// ================================================================================================

/// Account storage is composed of a variable number of index-addressable [StorageSlot]s up to
/// 255 slots in total.
///
/// Each slot has a type which defines its size and structure. Currently, the following types are
/// supported:
/// - [StorageSlot::Value]: contains a single [Word] of data (i.e., 32 bytes).
/// - [StorageSlot::Map]: contains a [StorageMap] which is a key-value map where both keys and
///   values are [Word]s. The value of a storage slot containing a map is the commitment to the
///   underlying map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountStorage {
    slots: Vec<StorageSlot>,
}

impl AccountStorage {
    /// The maximum number of storage slots allowed in an account storage.
    pub const MAX_NUM_STORAGE_SLOTS: usize = 255;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of account storage initialized with the provided items.
    pub fn new(slots: Vec<StorageSlot>) -> Result<AccountStorage, AccountError> {
        let num_slots = slots.len();

        if num_slots > Self::MAX_NUM_STORAGE_SLOTS {
            return Err(AccountError::StorageTooManySlots(num_slots as u64));
        }

        Ok(Self { slots })
    }

    // TODO Document.
    pub fn from_components(
        components: &[AccountComponent],
    ) -> Result<AccountStorage, AccountError> {
        let has_faucet_component =
            components.iter().any(|component| component.is_faucet_component());
        let mut storage_slots = if has_faucet_component {
            vec![StorageSlot::Value(Word::default()); 1]
        } else {
            vec![]
        };

        storage_slots
            .extend(components.iter().flat_map(|component| component.storage_slots()).cloned());

        Self::new(storage_slots)
    }

    pub fn from_components_with_faucet_metadata(
        components: &[AccountComponent],
        faucet_metadata: Word,
    ) -> Result<AccountStorage, AccountError> {
        let mut storage_slots = vec![StorageSlot::Value(faucet_metadata)];
        storage_slots
            .extend(components.iter().flat_map(|component| component.storage_slots()).cloned());

        Self::new(storage_slots)
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

    /// Converts storage slots of this account storage into a vector of field elements.
    ///
    /// This is done by first converting each procedure into exactly 8 elements as follows:
    /// ```text
    /// [STORAGE_SLOT_VALUE, storage_slot_type, 0, 0, 0]
    /// ```
    /// And then concatenating the resulting elements into a single vector.
    pub fn as_elements(&self) -> Vec<Felt> {
        slots_as_elements(self.slots())
    }

    /// Returns an item from the storage at the specified index.
    ///
    /// # Errors:
    /// - If the index is out of bounds
    pub fn get_item(&self, index: u8) -> Result<Digest, AccountError> {
        self.slots
            .get(index as usize)
            .ok_or(AccountError::StorageIndexOutOfBounds {
                max: self.slots.len() as u8,
                actual: index,
            })
            .map(|slot| slot.value().into())
    }

    /// Returns a map item from a map located in storage at the specified index.
    ///
    /// # Errors:
    /// - If the index is out of bounds
    /// - If the [StorageSlot] is not [StorageSlotType::Map]
    pub fn get_map_item(&self, index: u8, key: Word) -> Result<Word, AccountError> {
        match self.slots.get(index as usize).ok_or(AccountError::StorageIndexOutOfBounds {
            max: self.slots.len() as u8,
            actual: index,
        })? {
            StorageSlot::Map(ref map) => Ok(map.get_value(&Digest::from(key))),
            _ => Err(AccountError::StorageSlotNotMap(index)),
        }
    }

    /// Returns an [AccountStorageHeader] for this account storage.
    pub fn get_header(&self) -> AccountStorageHeader {
        AccountStorageHeader::new(
            self.slots.iter().map(|slot| (slot.slot_type(), slot.value())).collect(),
        )
    }

    // DATA MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Applies the provided delta to this account storage.
    ///
    /// # Errors:
    /// - If the updates violate storage constraints.
    pub(super) fn apply_delta(&mut self, delta: &AccountStorageDelta) -> Result<(), AccountError> {
        let len = self.slots.len() as u8;

        // update storage maps
        for (&idx, map) in delta.maps().iter() {
            let storage_slot = self
                .slots
                .get_mut(idx as usize)
                .ok_or(AccountError::StorageIndexOutOfBounds { max: len, actual: idx })?;

            let storage_map = match storage_slot {
                StorageSlot::Map(map) => map,
                _ => return Err(AccountError::StorageSlotNotMap(idx)),
            };

            storage_map.apply_delta(map);
        }

        // update storage values
        for (&idx, &value) in delta.values().iter() {
            self.set_item(idx, value)?;
        }

        Ok(())
    }

    /// Updates the value of the storage slot at the specified index.
    ///
    /// This method should be used only to update value slots. For updating values
    /// in storage maps, please see [AccountStorage::set_map_item()].
    ///
    /// # Errors:
    /// - If the index is out of bounds
    /// - If the [StorageSlot] is not [StorageSlotType::Value]
    pub fn set_item(&mut self, index: u8, value: Word) -> Result<Word, AccountError> {
        // check if index is in bounds
        let num_slots = self.slots.len();

        if index as usize >= num_slots {
            return Err(AccountError::StorageIndexOutOfBounds {
                max: self.slots.len() as u8,
                actual: index,
            });
        }

        let old_value = match self.slots[index as usize] {
            StorageSlot::Value(value) => value,
            // return an error if the type != Value
            _ => return Err(AccountError::StorageSlotNotValue(index)),
        };

        // update the value of the storage slot
        self.slots[index as usize] = StorageSlot::Value(value);

        Ok(old_value)
    }

    /// Updates the value of a key-value pair of a storage map at the specified index.
    ///
    /// This method should be used only to update storage maps. For updating values
    /// in storage slots, please see [AccountStorage::set_item()].
    ///
    /// # Errors:
    /// - If the index is out of bounds
    /// - If the [StorageSlot] is not [StorageSlotType::Map]
    pub fn set_map_item(
        &mut self,
        index: u8,
        key: Word,
        value: Word,
    ) -> Result<(Word, Word), AccountError> {
        // check if index is in bounds
        let num_slots = self.slots.len();

        if index as usize >= num_slots {
            return Err(AccountError::StorageIndexOutOfBounds {
                max: self.slots.len() as u8,
                actual: index,
            });
        }

        let storage_map = match self.slots[index as usize] {
            StorageSlot::Map(ref mut map) => map,
            _ => return Err(AccountError::StorageSlotNotMap(index)),
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
        target.write_u8(self.slots().len() as u8);
        target.write_many(self.slots());
    }

    fn get_size_hint(&self) -> usize {
        // Size of the serialized slot length.
        let u8_size = 0u8.get_size_hint();
        let mut size = u8_size;

        for slot in self.slots() {
            size += slot.get_size_hint();
        }

        size
    }
}

impl Deserializable for AccountStorage {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let num_slots = source.read_u8()? as usize;
        let slots = source.read_many::<StorageSlot>(num_slots)?;

        Self::new(slots).map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::{
        build_slots_commitment, AccountStorage, Deserializable, Serializable, StorageMap, Word,
    };
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

    #[test]
    fn test_account_storage_slots_commitment() {
        let storage = AccountStorage::mock();
        let storage_slots_commitment = build_slots_commitment(storage.slots());
        assert_eq!(storage_slots_commitment, storage.commitment())
    }
}
