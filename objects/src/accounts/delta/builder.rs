use alloc::vec::Vec;

use super::{AccountStorageDelta, StorageMapDelta, Word};
use crate::AccountDeltaError;

#[derive(Clone, Debug, Default)]
pub struct AccountStorageDeltaBuilder {
    pub cleared_items: Vec<u8>,
    pub updated_items: Vec<(u8, Word)>,
    pub updated_maps: Vec<(u8, StorageMapDelta)>,
}

impl AccountStorageDeltaBuilder {
    // CONSTRUCTORS
    // -------------------------------------------------------------------------------------------
    pub fn new() -> Self {
        Self::default()
    }

    // MODIFIERS
    // -------------------------------------------------------------------------------------------
    pub fn add_cleared_items<I>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = u8>,
    {
        self.cleared_items.extend(items);
        self
    }

    pub fn add_updated_items<I>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = (u8, Word)>,
    {
        self.updated_items.extend(items);
        self
    }

    pub fn add_updated_maps<I>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = (u8, StorageMapDelta)>,
    {
        self.updated_maps.extend(items);
        self
    }

    // BUILDERS
    // -------------------------------------------------------------------------------------------
    pub fn build(self) -> Result<AccountStorageDelta, AccountDeltaError> {
        let delta = AccountStorageDelta {
            cleared_items: self.cleared_items,
            updated_items: self.updated_items,
            updated_maps: self.updated_maps,
        };
        delta.validate()?;
        Ok(delta)
    }
}
