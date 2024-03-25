use alloc::vec::Vec;

use miden_objects::accounts::{AccountStorage, SlotItem};

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct AccountStorageBuilder {
    items: Vec<SlotItem>,
}

/// Builder for an `AccountStorage`, the builder can be configured and used multiple times.
impl AccountStorageBuilder {
    pub fn new() -> Self {
        Self { items: vec![] }
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

    pub fn build(&self) -> AccountStorage {
        AccountStorage::new(self.items.clone()).unwrap()
    }
}
