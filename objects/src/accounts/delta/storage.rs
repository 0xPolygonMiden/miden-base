use alloc::{
    collections::{btree_map::Entry, BTreeMap},
    string::ToString,
    vec::Vec,
};

use miden_crypto::EMPTY_WORD;

use super::{
    AccountDeltaError, ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable,
    Word,
};
use crate::Digest;

// CONSTANTS
// ================================================================================================

const IMMUTABLE_STORAGE_SLOT: u8 = u8::MAX;

// ACCOUNT STORAGE DELTA
// ================================================================================================

/// [AccountStorageDelta] stores the differences between two states of account storage.
///
/// The differences are represented as follows:
/// - item updates: represented by `cleared_items` and `updated_items` field.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccountStorageDelta {
    slots: BTreeMap<u8, Word>,
    maps: BTreeMap<u8, StorageMapDelta>,
}

impl AccountStorageDelta {
    /// Creates a new storage delta from the provided fields.
    pub fn new(
        slots: BTreeMap<u8, Word>,
        maps: BTreeMap<u8, StorageMapDelta>,
    ) -> Result<Self, AccountDeltaError> {
        let result = Self { slots, maps };
        result.validate()?;

        Ok(result)
    }

    /// Returns a reference to the updated slots in this storage delta.
    pub fn slots(&self) -> &BTreeMap<u8, Word> {
        &self.slots
    }

    /// Returns a reference to the updated maps in this storage delta.
    pub fn maps(&self) -> &BTreeMap<u8, StorageMapDelta> {
        &self.maps
    }

    /// Returns true if storage delta contains no updates.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty() && self.maps.is_empty()
    }

    /// Tracks a slot change
    pub fn slot_update(&mut self, slot_index: u8, new_slot_value: Word) {
        self.slots.insert(slot_index, new_slot_value);
    }

    /// Tracks a slot change
    pub fn maps_update(&mut self, slot_index: u8, key: Digest, new_value: Word) {
        self.maps.entry(slot_index).or_default().insert(key, new_value);
    }

    /// Merges another delta into this one, overwriting any existing values.
    pub fn merge(&mut self, other: Self) -> Result<(), AccountDeltaError> {
        self.slots.extend(other.slots);

        // merge maps
        for (slot, update) in other.maps.into_iter() {
            match self.maps.entry(slot) {
                Entry::Vacant(entry) => {
                    entry.insert(update);
                },
                Entry::Occupied(mut entry) => entry.get_mut().merge(update)?,
            }
        }

        self.validate()
    }

    /// Checks whether this storage delta is valid.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Any of updated items are at slot 255 (i.e., immutable slot).
    /// - Any of the updated slot is referenced from both maps (e.g., updated twice).
    fn validate(&self) -> Result<(), AccountDeltaError> {
        if self.slots.contains_key(&IMMUTABLE_STORAGE_SLOT)
            || self.maps.contains_key(&IMMUTABLE_STORAGE_SLOT)
        {
            return Err(AccountDeltaError::ImmutableStorageSlot(IMMUTABLE_STORAGE_SLOT as usize));
        }

        for slot in self.maps.keys() {
            if self.slots.contains_key(slot) {
                return Err(AccountDeltaError::DuplicateStorageItemUpdate(*slot as usize));
            }
        }

        Ok(())
    }
}

#[cfg(any(feature = "testing", test))]
impl AccountStorageDelta {
    /// Creates an [AccountStorageDelta] from the given iterators.
    pub fn from_iters(
        cleared_items: impl IntoIterator<Item = u8>,
        updated_items: impl IntoIterator<Item = (u8, Word)>,
        updated_maps: impl IntoIterator<Item = (u8, StorageMapDelta)>,
    ) -> Self {
        Self {
            slots: BTreeMap::from_iter(
                cleared_items.into_iter().map(|key| (key, EMPTY_WORD)).chain(updated_items),
            ),
            maps: BTreeMap::from_iter(updated_maps),
        }
    }
}

impl Serializable for AccountStorageDelta {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let cleared: Vec<u8> = self
            .slots
            .iter()
            .filter(|&(_, value)| (value == &EMPTY_WORD))
            .map(|(slot, _)| *slot)
            .collect();
        let updated: Vec<_> =
            self.slots.iter().filter(|&(_, value)| value != &EMPTY_WORD).collect();

        target.write_u8(cleared.len() as u8);
        target.write_many(cleared.iter());

        target.write_u8(updated.len() as u8);
        target.write_many(updated.iter());

        target.write_u8(self.maps.len() as u8);
        target.write_many(self.maps.iter());
    }
}

impl Deserializable for AccountStorageDelta {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let num_cleared_items = source.read_u8()? as usize;
        let cleared_items: Vec<u8> = source.read_many(num_cleared_items)?;

        let num_updated_items = source.read_u8()? as usize;
        let updated_items: Vec<(u8, Word)> = source.read_many(num_updated_items)?;

        let slots = cleared_items
            .into_iter()
            .map(|slot| (slot, EMPTY_WORD))
            .chain(updated_items)
            .collect();

        let num_maps = source.read_u8()? as usize;
        let maps = source.read_many::<(u8, StorageMapDelta)>(num_maps)?.into_iter().collect();

        Self::new(slots, maps).map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// STORAGE MAP DELTA
// ================================================================================================

/// [StorageMapDelta] stores the differences between two states of account storage maps.
///
/// The differences are represented as leaf updates: a map of updated item key ([Digest]) to
/// value ([Word]). For cleared items the value is [EMPTY_WORD].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StorageMapDelta(BTreeMap<Digest, Word>);

impl StorageMapDelta {
    /// Creates a new storage map delta from the provided leaves.
    pub fn new(map: BTreeMap<Digest, Word>) -> Result<Self, AccountDeltaError> {
        let delta = Self(map);
        delta.validate()?;

        Ok(delta)
    }

    /// Returns a reference to the updated leaves in this storage map delta.
    pub fn leaves(&self) -> &BTreeMap<Digest, Word> {
        &self.0
    }

    /// Inserts an item into the storage map delta.
    pub fn insert(&mut self, key: Digest, value: Word) {
        self.0.insert(key, value);
    }

    /// Returns true if storage map delta contains no updates.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Merge `other` into this delta, giving precedence to `other`.
    pub fn merge(&mut self, other: Self) -> Result<(), AccountDeltaError> {
        // Aggregate the changes into a map such that `other` overwrites self.
        self.0.extend(other.0);

        // Validate the resulted delta.
        self.validate()
    }

    /// Validates the storage map delta.
    fn validate(&self) -> Result<(), AccountDeltaError> {
        if self.leaves().len() > u16::MAX as usize {
            return Err(AccountDeltaError::TooManyStorageItems {
                actual: self.leaves().len(),
                max: u16::MAX as usize,
            });
        }

        Ok(())
    }
}

#[cfg(any(feature = "testing", test))]
impl StorageMapDelta {
    /// Creates a new [StorageMapDelta] from the provided iterators.
    pub fn from_iters(
        cleared_leaves: impl IntoIterator<Item = Word>,
        updated_leaves: impl IntoIterator<Item = (Word, Word)>,
    ) -> Self {
        Self(BTreeMap::from_iter(
            cleared_leaves
                .into_iter()
                .map(|key| (key.into(), EMPTY_WORD))
                .chain(updated_leaves.into_iter().map(|(key, value)| (key.into(), value))),
        ))
    }
}

impl Serializable for StorageMapDelta {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let cleared: Vec<&Digest> = self
            .0
            .iter()
            .filter(|&(_, value)| value == &EMPTY_WORD)
            .map(|(key, _)| key)
            .collect();

        let updated: Vec<_> = self.0.iter().filter(|&(_, value)| value != &EMPTY_WORD).collect();

        target.write_usize(cleared.len());
        target.write_many(cleared.iter());

        target.write_usize(updated.len());
        target.write_many(updated.iter());
    }
}

impl Deserializable for StorageMapDelta {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let cleared_count = source.read_usize()?;
        let cleared: Vec<Digest> = source.read_many(cleared_count)?;

        let updated_count = source.read_usize()?;
        let updated: Vec<(Digest, Word)> = source.read_many(updated_count)?;

        let map =
            BTreeMap::from_iter(cleared.into_iter().map(|key| (key, EMPTY_WORD)).chain(updated));

        Self::new(map).map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// ACCOUNT STORAGE DELTA BUILDER
// ================================================================================================

#[derive(Clone, Debug, Default)]
pub struct AccountStorageDeltaBuilder {
    slots: BTreeMap<u8, Word>,
    maps: BTreeMap<u8, StorageMapDelta>,
}

impl AccountStorageDeltaBuilder {
    // MODIFIERS
    // -------------------------------------------------------------------------------------------

    pub fn add_cleared_items(mut self, items: impl IntoIterator<Item = u8>) -> Self {
        self.slots.extend(items.into_iter().map(|slot| (slot, EMPTY_WORD)));
        self
    }

    pub fn add_updated_items(mut self, items: impl IntoIterator<Item = (u8, Word)>) -> Self {
        self.slots.extend(items);
        self
    }

    pub fn add_updated_maps(
        mut self,
        items: impl IntoIterator<Item = (u8, StorageMapDelta)>,
    ) -> Self {
        self.maps.extend(items);
        self
    }

    // BUILDERS
    // -------------------------------------------------------------------------------------------

    pub fn build(self) -> Result<AccountStorageDelta, AccountDeltaError> {
        let delta = AccountStorageDelta { slots: self.slots, maps: self.maps };
        delta.validate()?;

        Ok(delta)
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::{AccountStorageDelta, Deserializable, Serializable};
    use crate::{
        accounts::{delta::AccountStorageDeltaBuilder, StorageMapDelta},
        ONE, ZERO,
    };

    #[test]
    fn account_storage_delta_validation() {
        let delta = AccountStorageDelta::from_iters(
            [1, 2, 3],
            [(4, [ONE, ONE, ONE, ONE]), (5, [ONE, ONE, ONE, ZERO])],
            [],
        );
        assert!(delta.validate().is_ok());

        let bytes = delta.to_bytes();
        assert_eq!(AccountStorageDelta::read_from_bytes(&bytes), Ok(delta));

        // invalid index in cleared items
        let delta = AccountStorageDelta::from_iters([1, 2, 255], [], []);
        assert!(delta.validate().is_err());

        let bytes = delta.to_bytes();
        assert!(AccountStorageDelta::read_from_bytes(&bytes).is_err());

        let bytes = delta.to_bytes();
        assert!(AccountStorageDelta::read_from_bytes(&bytes).is_err());

        // invalid index in updated items
        let delta = AccountStorageDelta::from_iters(
            [],
            [(4, [ONE, ONE, ONE, ONE]), (255, [ONE, ONE, ONE, ZERO])],
            [],
        );
        assert!(delta.validate().is_err());

        let bytes = delta.to_bytes();
        assert!(AccountStorageDelta::read_from_bytes(&bytes).is_err());

        let bytes = delta.to_bytes();
        assert!(AccountStorageDelta::read_from_bytes(&bytes).is_err());

        // duplicate across cleared items and maps
        let delta = AccountStorageDelta::from_iters(
            [1, 2, 3],
            [(2, [ONE, ONE, ONE, ONE]), (5, [ONE, ONE, ONE, ZERO])],
            [(1, StorageMapDelta::default())],
        );
        assert!(delta.validate().is_err());

        let bytes = delta.to_bytes();
        assert!(AccountStorageDelta::read_from_bytes(&bytes).is_err());

        // duplicate across updated items and maps
        let delta = AccountStorageDelta::from_iters(
            [1, 3],
            [(2, [ONE, ONE, ONE, ONE]), (5, [ONE, ONE, ONE, ZERO])],
            [(2, StorageMapDelta::default())],
        );
        assert!(delta.validate().is_err());

        let bytes = delta.to_bytes();
        assert!(AccountStorageDelta::read_from_bytes(&bytes).is_err());
    }

    #[test]
    fn test_is_empty() {
        let storage_delta = AccountStorageDelta::default();
        assert!(storage_delta.is_empty());

        let storage_delta = AccountStorageDelta::from_iters([1], [], []);
        assert!(!storage_delta.is_empty());

        let storage_delta = AccountStorageDelta::from_iters([], [(2, [ONE, ONE, ONE, ONE])], []);
        assert!(!storage_delta.is_empty());

        let storage_delta =
            AccountStorageDelta::from_iters([], [], [(3, StorageMapDelta::default())]);
        assert!(!storage_delta.is_empty());
    }

    #[test]
    fn test_serde_account_storage_delta() {
        let storage_delta = AccountStorageDelta::default();
        let serialized = storage_delta.to_bytes();
        let deserialized = AccountStorageDelta::read_from_bytes(&serialized).unwrap();
        assert_eq!(deserialized, storage_delta);

        let storage_delta = AccountStorageDelta::from_iters([1], [], []);
        let serialized = storage_delta.to_bytes();
        let deserialized = AccountStorageDelta::read_from_bytes(&serialized).unwrap();
        assert_eq!(deserialized, storage_delta);

        let storage_delta = AccountStorageDelta::from_iters([], [(2, [ONE, ONE, ONE, ONE])], []);
        let serialized = storage_delta.to_bytes();
        let deserialized = AccountStorageDelta::read_from_bytes(&serialized).unwrap();
        assert_eq!(deserialized, storage_delta);

        let storage_delta =
            AccountStorageDelta::from_iters([], [], [(3, StorageMapDelta::default())]);
        let serialized = storage_delta.to_bytes();
        let deserialized = AccountStorageDelta::read_from_bytes(&serialized).unwrap();
        assert_eq!(deserialized, storage_delta);
    }

    #[test]
    fn test_serde_storage_map_delta() {
        let storage_map_delta = StorageMapDelta::default();
        let serialized = storage_map_delta.to_bytes();
        let deserialized = StorageMapDelta::read_from_bytes(&serialized).unwrap();
        assert_eq!(deserialized, storage_map_delta);

        let storage_map_delta = StorageMapDelta::from_iters([[ONE, ONE, ONE, ONE]], []);
        let serialized = storage_map_delta.to_bytes();
        let deserialized = StorageMapDelta::read_from_bytes(&serialized).unwrap();
        assert_eq!(deserialized, storage_map_delta);

        let storage_map_delta =
            StorageMapDelta::from_iters([], [([ZERO, ZERO, ZERO, ZERO], [ONE, ONE, ONE, ONE])]);
        let serialized = storage_map_delta.to_bytes();
        let deserialized = StorageMapDelta::read_from_bytes(&serialized).unwrap();
        assert_eq!(deserialized, storage_map_delta);
    }

    #[rstest::rstest]
    #[case::some_some(Some(1), Some(2), Some(2))]
    #[case::none_some(None, Some(2), Some(2))]
    #[case::some_none(Some(1), None, None)]
    #[test]
    fn merge_items(#[case] x: Option<u64>, #[case] y: Option<u64>, #[case] expected: Option<u64>) {
        /// Creates a delta containing the item as an update if Some, else with the item cleared.
        fn create_delta(item: Option<u64>) -> AccountStorageDelta {
            const SLOT: u8 = 123;
            let item = item.map(|x| (SLOT, [vm_core::Felt::new(x), ZERO, ZERO, ZERO]));

            AccountStorageDeltaBuilder::default()
                .add_cleared_items(item.is_none().then_some(SLOT))
                .add_updated_items(item)
                .build()
                .unwrap()
        }

        let mut delta_x = create_delta(x);
        let delta_y = create_delta(y);
        let expected = create_delta(expected);

        delta_x.merge(delta_y).unwrap();

        assert_eq!(delta_x, expected);
    }

    #[rstest::rstest]
    #[case::some_some(Some(1), Some(2), Some(2))]
    #[case::none_some(None, Some(2), Some(2))]
    #[case::some_none(Some(1), None, None)]
    #[test]
    fn merge_maps(#[case] x: Option<u64>, #[case] y: Option<u64>, #[case] expected: Option<u64>) {
        fn create_delta(value: Option<u64>) -> StorageMapDelta {
            let key = [vm_core::Felt::new(10), ZERO, ZERO, ZERO];
            match value {
                Some(value) => StorageMapDelta::from_iters(
                    [],
                    [(key, [vm_core::Felt::new(value), ZERO, ZERO, ZERO])],
                ),
                None => StorageMapDelta::from_iters([key], []),
            }
        }

        let mut delta_x = create_delta(x);
        let delta_y = create_delta(y);
        let expected = create_delta(expected);

        delta_x.merge(delta_y).unwrap();

        assert_eq!(delta_x, expected);
    }
}
