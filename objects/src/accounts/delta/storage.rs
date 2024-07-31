use std::collections::HashSet;

use alloc::{string::ToString, vec::Vec};
use vm_core::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::DeserializationError;

use crate::{
    accounts::{AccountStorage, StorageSlot},
    AccountDeltaError,
};

// ACCOUNT STORAGE DELTA
// ================================================================================================

/// [AccountStorageDelta] stores the differences between two states of account storage.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccountStorageDelta {
    items: Vec<(u8, StorageSlot)>,
}

impl AccountStorageDelta {
    pub const MAX_MUTABLE_STORAGE_SLOT_IDX: u8 = 255;

    /// Returns a new instance of an [AccountStorageDelta].
    pub fn new(items: &[(u8, StorageSlot)]) -> Result<Self, AccountDeltaError> {
        let delta = Self { items: items.to_vec() };
        let _ = delta.validate();
        Ok(delta)
    }

    /// Returns a reference to the updated items of this storage delta.
    pub fn items(&self) -> &[(u8, StorageSlot)] {
        &self.items
    }

    /// Returns true if storage delta contains no updates.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Merges another delta into this one, overwriting any existing values.
    pub fn merge(&self, other: Self) -> Result<Self, AccountDeltaError> {
        // validate both deltas
        self.validate()?;
        other.validate()?;

        let mut items = self.items().to_vec();
        let other_items = other.items();

        // iterate over other_items and merge;
        for (other_idx, other_storage_slot) in other_items {
            if let Some(pos) = items.iter().position(|(idx, _)| idx == other_idx) {
                // index already exists replace the existing update with the new update.
                items[pos] = (*other_idx, other_storage_slot.clone());
            } else {
                // index does not exist add new item
                items.push((*other_idx, other_storage_slot.clone()))
            }
        }

        Ok(Self::new(&items)?)
    }

    /// Checks wether this storage delta is valid.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The number of updated items is greater than 255 (i.g., too many updates)
    /// - The index of the update is out of bounds
    /// - Update index is referenced more than once (e.g., updated twice)
    pub fn validate(&self) -> Result<(), AccountDeltaError> {
        let num_items = self.items.len();

        if num_items > AccountStorage::NUM_STORAGE_SLOTS {
            return Err(AccountDeltaError::TooManyUpdatedStorageItems {
                actual: num_items,
                max: AccountStorage::NUM_STORAGE_SLOTS,
            });
        }

        let mut seen_indices = HashSet::new();
        for (idx, _) in self.items.iter() {
            // make sure index is in bounds
            if idx > &Self::MAX_MUTABLE_STORAGE_SLOT_IDX {
                return Err(AccountDeltaError::UpdateIndexOutOfBounds(*idx as usize));
            }

            // make sure no slot has been updated twice
            if !seen_indices.insert(*idx) {
                return Err(AccountDeltaError::DuplicateStorageItemUpdate(*idx as usize));
            }
        }

        Ok(())
    }
}

// Serialization
// ================================================================================================

impl Serializable for AccountStorageDelta {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        assert!(
            self.items.len() <= AccountStorage::NUM_STORAGE_SLOTS as usize,
            "too many updated storage items"
        );
        target.write_u8(self.items.len() as u8);
        target.write_many(self.items());
    }
}

impl Deserializable for AccountStorageDelta {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        // deserialize and validate cleared items
        let num_items = source.read_u8()? as usize;
        let items = source.read_many::<(u8, StorageSlot)>(num_items)?;
        let delta = AccountStorageDelta::new(&items)
            .map_err(|err| DeserializationError::InvalidValue(err.to_string()))?;
        Ok(delta)
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use vm_core::{
        utils::{Deserializable, Serializable},
        ONE, ZERO,
    };

    use crate::accounts::StorageSlot;

    use super::AccountStorageDelta;

    fn build_delta() -> AccountStorageDelta {
        // build items
        let item_0 = (0, StorageSlot::Value([ONE, ZERO, ONE, ZERO]));
        let item_1 = (1, StorageSlot::Value([ZERO, ONE, ZERO, ONE]));
        let item_2 = (2, StorageSlot::Map(()));
        let items = vec![item_0, item_1, item_2];

        // build delta
        AccountStorageDelta::new(&items)
    }

    #[test]
    fn account_storage_delta_validation() {
        // build delta
        let delta = build_delta();

        assert!(delta.validate().is_ok())
    }

    #[test]
    fn account_storage_delta_serde() {
        // build delta
        let delta = build_delta();

        // make sure that delta is valid
        delta.validate();

        // serde delta
        let serialized = delta.to_bytes();
        let deserialized = AccountStorageDelta::read_from_bytes(&serialized).unwrap();

        // check that serde was properly done
        assert_eq!(deserialized, delta);
    }
}
