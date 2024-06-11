use alloc::vec::Vec;

use crate::accounts::{AccountStorage, SlotItem, StorageMap};

#[derive(Default, Debug, Clone)]
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

    #[allow(dead_code)]
    pub fn add_map(&mut self, map: StorageMap) -> &mut Self {
        self.maps.push(map);
        self
    }

    pub fn build(&self) -> AccountStorage {
        AccountStorage::new(self.items.clone(), self.maps.clone()).unwrap()
    }
}
