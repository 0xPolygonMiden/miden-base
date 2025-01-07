use alloc::collections::BTreeMap;

use super::{StoragePlaceholder, StorageValue};

/// Represents the data required to initialize storage entries when instantiating an
/// [AccountComponent](crate::accounts::AccountComponent).
#[derive(Clone, Debug, Default)]
pub struct InitStorageData {
    /// A mapping of storage placeholder names to their corresponding template values.
    storage_placeholders: BTreeMap<StoragePlaceholder, StorageValue>,
}

impl InitStorageData {
    /// Creates a new instance of [InitStorageData].
    ///
    /// # Parameters
    ///
    /// - `entries`: An iterable collection of key-value pairs.
    pub fn new(entries: impl IntoIterator<Item = (StoragePlaceholder, StorageValue)>) -> Self {
        InitStorageData {
            storage_placeholders: entries.into_iter().collect(),
        }
    }

    /// Retrieves a reference to the template values.
    pub fn inner(&self) -> &BTreeMap<StoragePlaceholder, StorageValue> {
        &self.storage_placeholders
    }

    /// Returns a reference to the [StorageValue] corresponding to the key, or [`Option::None`]
    /// if the key is not present.
    pub fn get(&self, key: &StoragePlaceholder) -> Option<&StorageValue> {
        self.storage_placeholders.get(key)
    }
}
