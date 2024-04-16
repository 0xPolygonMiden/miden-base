use alloc::{string::ToString, vec::Vec};

use super::{
    AccountDeltaError, ByteReader, ByteWriter, Deserializable, DeserializationError, Felt,
    Serializable, Word,
};

// CONSTANTS
// ================================================================================================

const MAX_MUTABLE_STORAGE_SLOT_IDX: u8 = 254;

// ACCOUNT STORAGE DELTA
// ================================================================================================

/// [AccountStorageDelta] stores the differences between two states of account storage.
///
/// The differences are represented as follows:
/// - item updates: represented by `cleared_items` and `updated_items` field.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccountStorageDelta {
    pub cleared_items: Vec<u8>,
    pub updated_items: Vec<(u8, Word)>,
    pub updated_maps: Vec<(u8, StorageMapDelta)>,
}

impl AccountStorageDelta {
    /// Checks whether this storage delta is valid.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The number of cleared or updated items is greater than 255.
    /// - Any of cleared or updated items are at slot 255 (i.e., immutable slot).
    /// - Any of the cleared or updated items is referenced more than once (e.g., updated twice).
    /// - There is a storage map delta without a corresponding storage item update.
    pub fn validate(&self) -> Result<(), AccountDeltaError> {
        let num_cleared_items = self.cleared_items.len();
        let num_updated_items = self.updated_items.len();

        if num_cleared_items > u8::MAX as usize {
            return Err(AccountDeltaError::TooManyClearedStorageItems {
                actual: num_cleared_items,
                max: u8::MAX as usize,
            });
        } else if num_updated_items > u8::MAX as usize {
            return Err(AccountDeltaError::TooManyRemovedAssets {
                actual: num_updated_items,
                max: u8::MAX as usize,
            });
        }

        // make sure cleared items vector does not contain errors
        for (pos, &idx) in self.cleared_items.iter().enumerate() {
            if idx > MAX_MUTABLE_STORAGE_SLOT_IDX {
                return Err(AccountDeltaError::ImmutableStorageSlot(idx as usize));
            }

            if self.cleared_items[..pos].contains(&idx) {
                return Err(AccountDeltaError::DuplicateStorageItemUpdate(idx as usize));
            }
        }

        // make sure updates items vector does not contain errors
        for (pos, (idx, _)) in self.updated_items.iter().enumerate() {
            if *idx > MAX_MUTABLE_STORAGE_SLOT_IDX {
                return Err(AccountDeltaError::ImmutableStorageSlot(*idx as usize));
            }

            if self.cleared_items.contains(idx) {
                return Err(AccountDeltaError::DuplicateStorageItemUpdate(*idx as usize));
            }

            if self.updated_items[..pos].iter().any(|x| x.0 == *idx) {
                return Err(AccountDeltaError::DuplicateStorageItemUpdate(*idx as usize));
            }
        }

        // make sure storage map deltas are valid
        for (index, storage_map_delta) in self.updated_maps.iter() {
            if index > &MAX_MUTABLE_STORAGE_SLOT_IDX {
                return Err(AccountDeltaError::ImmutableStorageSlot(*index as usize));
            }
            // for every storage map delta there should be one corresponding storage item update
            if !self.updated_items.iter().any(|(idx, _)| idx == index) {
                return Err(AccountDeltaError::StorageMapDeltaWithoutStorageItemChange(
                    *index as usize,
                ));
            }
            if !storage_map_delta.is_empty() {
                storage_map_delta.validate()?;
            }
        }

        Ok(())
    }

    /// Returns true if storage delta contains no updates.
    pub fn is_empty(&self) -> bool {
        self.cleared_items.is_empty() && self.updated_items.is_empty()
    }
}

impl Serializable for AccountStorageDelta {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        assert!(self.cleared_items.len() <= u8::MAX as usize, "too many cleared storage items");
        target.write_u8(self.cleared_items.len() as u8);
        for idx in self.cleared_items.iter() {
            idx.write_into(target);
        }

        assert!(self.updated_items.len() <= u8::MAX as usize, "too many updated storage items");
        target.write_u8(self.updated_items.len() as u8);
        for (idx, value) in self.updated_items.iter() {
            idx.write_into(target);
            value.write_into(target);
        }

        assert!(self.updated_maps.len() <= u8::MAX as usize, "too many updated storage maps");
        target.write_u8(self.updated_maps.len() as u8);
        self.updated_maps.write_into(target);
    }
}

impl Deserializable for AccountStorageDelta {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        // deserialize and validate cleared items
        let num_cleared_items = source.read_u8()? as usize;
        let mut cleared_items = Vec::with_capacity(num_cleared_items);
        for _ in 0..num_cleared_items {
            let idx = source.read_u8()?;

            // make sure index is valid
            if idx > MAX_MUTABLE_STORAGE_SLOT_IDX {
                return Err(DeserializationError::InvalidValue(
                    "immutable storage item cleared".to_string(),
                ));
            }

            // make sure the same item hasn't been cleared before
            if cleared_items.contains(&idx) {
                return Err(DeserializationError::InvalidValue(
                    "storage item cleared more than once".to_string(),
                ));
            }

            cleared_items.push(idx);
        }

        // deserialize and validate updated items
        let num_updated_items = source.read_u8()? as usize;
        let mut updated_items: Vec<(u8, Word)> = Vec::with_capacity(num_updated_items);
        for _ in 0..num_updated_items {
            let idx = source.read_u8()?;
            let value = Word::read_from(source)?;

            // make sure index is valid
            if idx > MAX_MUTABLE_STORAGE_SLOT_IDX {
                return Err(DeserializationError::InvalidValue(
                    "immutable storage item updated".to_string(),
                ));
            }

            // make sure the same item hasn't been updated before
            if updated_items.iter().any(|x| x.0 == idx) {
                return Err(DeserializationError::InvalidValue(
                    "storage item updated more than once".to_string(),
                ));
            }

            // make sure the storage item hasn't been cleared in the same delta
            if cleared_items.contains(&idx) {
                return Err(DeserializationError::InvalidValue(
                    "storage item both cleared and updated".to_string(),
                ));
            }

            updated_items.push((idx, value));
        }

        // deserialize and validate storage map deltas
        let num_updated_maps = source.read_u8()? as usize;
        let mut updated_maps = Vec::with_capacity(num_updated_maps);
        for _ in 0..num_updated_maps {
            let idx = source.read_u8()?;
            let value = StorageMapDelta::read_from(source)?;
            updated_maps.push((idx, value));
        }

        Ok(Self {
            cleared_items,
            updated_items,
            updated_maps,
        })
    }
}

// STORAGE MAP DELTA
// ================================================================================================

/// [StorageMapDelta] stores the differences between two states of account storage maps.
///
/// The differences are represented as follows:
/// - leave updates: represented by `cleared_leaves` and `updated_leaves` field.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StorageMapDelta {
    pub cleared_leaves: Vec<Word>,
    pub updated_leaves: Vec<(Word, Word)>,
}

impl StorageMapDelta {
    /// Creates a new storage map delta from the provided cleared and updated leaves.
    pub fn from(cleared_leaves: Vec<Word>, updated_leaves: Vec<(Word, Word)>) -> Self {
        StorageMapDelta { cleared_leaves, updated_leaves }
    }

    /// Checks whether this storage map delta is valid.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Any of the cleared or updated leaves is referenced more than once (e.g., updated twice).
    pub fn validate(&self) -> Result<(), AccountDeltaError> {
        // we add all keys to a single vector and sort them to check for duplicates
        // we don't use a hash set because we want to use no-std compatible code
        let mut all_updated_keys =
            Vec::with_capacity(self.cleared_leaves.len() + self.updated_leaves.len());

        // in order to sort the keys, we need to convert them to integers
        for &key in &self.cleared_leaves {
            let key_as_ints = key.iter().map(|x| x.as_int()).collect::<Vec<_>>();
            all_updated_keys.push(key_as_ints);
        }
        for &(key, _) in &self.updated_leaves {
            let key_as_ints = key.iter().map(|x| x.as_int()).collect::<Vec<_>>();
            all_updated_keys.push(key_as_ints);
        }

        all_updated_keys.sort();

        for key in all_updated_keys.windows(2) {
            if key[0] == key[1] {
                let mut iter = key[0].iter().map(|&x| Felt::new(x));
                // we know that the key is 4 elements long
                let key_duplicate = Word::from([
                    iter.next().unwrap(),
                    iter.next().unwrap(),
                    iter.next().unwrap(),
                    iter.next().unwrap(),
                ]);
                return Err(AccountDeltaError::DuplicateStorageMapLeaf {
                    key: key_duplicate.into(),
                });
            }
        }

        Ok(())
    }

    /// Returns true if storage map delta contains no updates.
    pub fn is_empty(&self) -> bool {
        self.cleared_leaves.is_empty() && self.updated_leaves.is_empty()
    }
}

impl Serializable for StorageMapDelta {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.cleared_leaves.write_into(target);
        self.updated_leaves.write_into(target);
    }
}

impl Deserializable for StorageMapDelta {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let cleared_leaves = Vec::<_>::read_from(source)?;
        let updated_leaves = Vec::<_>::read_from(source)?;
        Ok(Self { cleared_leaves, updated_leaves })
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::{AccountStorageDelta, Deserializable, Serializable};
    use crate::{ONE, ZERO};

    #[test]
    fn account_storage_delta_validation() {
        let delta = AccountStorageDelta {
            cleared_items: vec![1, 2, 3],
            updated_items: vec![(4, [ONE, ONE, ONE, ONE]), (5, [ONE, ONE, ONE, ZERO])],
            updated_maps: vec![],
        };
        assert!(delta.validate().is_ok());

        let bytes = delta.to_bytes();
        assert_eq!(AccountStorageDelta::read_from_bytes(&bytes), Ok(delta));

        // invalid index in cleared items
        let delta = AccountStorageDelta {
            cleared_items: vec![1, 2, 255],
            updated_items: vec![],
            updated_maps: vec![],
        };
        assert!(delta.validate().is_err());

        let bytes = delta.to_bytes();
        assert!(AccountStorageDelta::read_from_bytes(&bytes).is_err());

        // duplicate in cleared items
        let delta = AccountStorageDelta {
            cleared_items: vec![1, 2, 1],
            updated_items: vec![],
            updated_maps: vec![],
        };
        assert!(delta.validate().is_err());

        let bytes = delta.to_bytes();
        assert!(AccountStorageDelta::read_from_bytes(&bytes).is_err());

        // invalid index in updated items
        let delta = AccountStorageDelta {
            cleared_items: vec![],
            updated_items: vec![(4, [ONE, ONE, ONE, ONE]), (255, [ONE, ONE, ONE, ZERO])],
            updated_maps: vec![],
        };
        assert!(delta.validate().is_err());

        let bytes = delta.to_bytes();
        assert!(AccountStorageDelta::read_from_bytes(&bytes).is_err());

        // duplicate in updated items
        let delta = AccountStorageDelta {
            cleared_items: vec![],
            updated_items: vec![
                (4, [ONE, ONE, ONE, ONE]),
                (5, [ONE, ONE, ONE, ZERO]),
                (4, [ONE, ONE, ZERO, ZERO]),
            ],
            updated_maps: vec![],
        };
        assert!(delta.validate().is_err());

        let bytes = delta.to_bytes();
        assert!(AccountStorageDelta::read_from_bytes(&bytes).is_err());

        // duplicate across cleared and updated items
        let delta = AccountStorageDelta {
            cleared_items: vec![1, 2, 3],
            updated_items: vec![(2, [ONE, ONE, ONE, ONE]), (5, [ONE, ONE, ONE, ZERO])],
            updated_maps: vec![],
        };
        assert!(delta.validate().is_err());

        let bytes = delta.to_bytes();
        assert!(AccountStorageDelta::read_from_bytes(&bytes).is_err());
    }
}
