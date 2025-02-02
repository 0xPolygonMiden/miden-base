use alloc::collections::BTreeMap;

use super::{StoragePlaceholder, StorageValue};

/// Represents the data required to initialize storage entries when instantiating an
/// [AccountComponent](crate::account::AccountComponent) from a
/// [template](crate::account::AccountComponentTemplate).
#[derive(Clone, Debug, Default)]
pub struct InitStorageData {
    /// A mapping of storage placeholder names to their corresponding storage values.
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

    /// Retrieves a reference to the storage placeholders.
    pub fn placeholders(&self) -> &BTreeMap<StoragePlaceholder, StorageValue> {
        &self.storage_placeholders
    }

    /// Returns a reference to the [StorageValue] corresponding to the placeholder, or
    /// [`Option::None`] if the placeholder is not present.
    pub fn get(&self, key: &StoragePlaceholder) -> Option<&StorageValue> {
        self.storage_placeholders.get(key)
    }
}
