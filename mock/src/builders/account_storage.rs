use alloc::vec::Vec;

use miden_objects::accounts::{AccountStorage, SlotItem, StorageMap};

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct AccountStorageBuilder {
    items: Vec<SlotItem>,
    maps: Vec<StorageMap>,
}

/// Builder for an `AccountStorage`, the builder can be configured and used multiple times.
impl AccountStorageBuilder {
    pub fn new() -> Self {
        Self { items: vec![], maps: vec![] }
    }

    pub fn add_item(&mut self, item: SlotItem) -> &mut Self {
        self.items.push(item);
        self
    }

    pub fn add_items<I: IntoIterator<Item = SlotItem>>(&mut self, items: I) -> &mut Self {
        for item in items.into_iter() {
            self.add_item(item);
        }
        self
    }

    pub fn add_map(&mut self, map: StorageMap) -> &mut Self {
        self.maps.push(map);
        self
    }

    pub fn add_maps<I: IntoIterator<Item = StorageMap>>(&mut self, maps: I) -> &mut Self {
        self.maps.extend(maps);
        self
    }

    pub fn build(&self) -> AccountStorage {
        AccountStorage::new(self.items.clone(), self.maps.clone()).unwrap()
    }
}
