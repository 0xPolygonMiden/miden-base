use alloc::vec::Vec;

use super::AccountStorageDelta;
use crate::{accounts::StorageSlot, AccountDeltaError};

#[derive(Clone, Debug, Default)]
pub struct AccountStorageDeltaBuilder {
    pub items: Vec<(u8, StorageSlot)>,
}

impl AccountStorageDeltaBuilder {
    // CONSTRUCTORS
    // -------------------------------------------------------------------------------------------
    pub fn new() -> Self {
        Self::default()
    }

    // MODIFIERS
    // -------------------------------------------------------------------------------------------
    pub fn add_items<I>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = (u8, StorageSlot)>,
    {
        self.items.extend(items);
        self
    }

    // BUILDERS
    // -------------------------------------------------------------------------------------------
    pub fn build(self) -> Result<AccountStorageDelta, AccountDeltaError> {
        let delta = AccountStorageDelta::new(&self.items)?;
        Ok(delta)
    }
}
