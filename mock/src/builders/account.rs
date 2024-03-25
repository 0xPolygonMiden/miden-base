use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::{
    accounts::{Account, AccountStorage, AccountType, SlotItem},
    assets::{Asset, AssetVault},
    Felt, Word, ZERO,
};
use rand::Rng;

use crate::{
    builders::{str_to_account_code, AccountBuilderError, AccountIdBuilder, AccountStorageBuilder},
    mock::account::DEFAULT_ACCOUNT_CODE,
};

/// Builder for an `Account`, the builder allows for a fluent API to construct an account. Each
/// account needs a unique builder.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct AccountBuilder<T> {
    assets: Vec<Asset>,
    storage_builder: AccountStorageBuilder,
    code: String,
    nonce: Felt,
    account_id_builder: AccountIdBuilder<T>,
}

impl<T: Rng> AccountBuilder<T> {
    pub fn new(rng: T) -> Self {
        Self {
            assets: vec![],
            storage_builder: AccountStorageBuilder::new(),
            code: DEFAULT_ACCOUNT_CODE.to_string(),
            nonce: ZERO,
            account_id_builder: AccountIdBuilder::new(rng),
        }
    }

    pub fn add_asset(mut self, asset: Asset) -> Self {
        self.assets.push(asset);
        self
    }

    pub fn add_assets<I: IntoIterator<Item = Asset>>(mut self, assets: I) -> Self {
        for asset in assets.into_iter() {
            self.assets.push(asset);
        }
        self
    }

    pub fn add_storage_item(mut self, item: SlotItem) -> Self {
        self.storage_builder.add_item(item);
        self
    }

    pub fn add_storage_items<I: IntoIterator<Item = SlotItem>>(mut self, items: I) -> Self {
        self.storage_builder.add_items(items);
        self
    }

    pub fn code<C: AsRef<str>>(mut self, code: C) -> Self {
        self.code = code.as_ref().to_string();
        self
    }

    pub fn nonce(mut self, nonce: Felt) -> Self {
        self.nonce = nonce;
        self
    }

    pub fn account_type(mut self, account_type: AccountType) -> Self {
        self.account_id_builder.account_type(account_type);
        self
    }

    pub fn on_chain(mut self, on_chain: bool) -> Self {
        self.account_id_builder.on_chain(on_chain);
        self
    }

    pub fn build(mut self) -> Result<Account, AccountBuilderError> {
        let vault = AssetVault::new(&self.assets).map_err(AccountBuilderError::AssetVaultError)?;
        let storage = self.storage_builder.build();
        self.account_id_builder.code(&self.code);
        self.account_id_builder.storage_root(storage.root());
        let account_id = self.account_id_builder.build()?;
        let account_code =
            str_to_account_code(&self.code).map_err(AccountBuilderError::AccountError)?;
        Ok(Account::new(account_id, vault, storage, account_code, self.nonce))
    }

    /// Build an account using the provided `seed`.
    pub fn with_seed(mut self, seed: Word) -> Result<Account, AccountBuilderError> {
        let vault = AssetVault::new(&self.assets).map_err(AccountBuilderError::AssetVaultError)?;
        let storage = self.storage_builder.build();
        self.account_id_builder.code(&self.code);
        self.account_id_builder.storage_root(storage.root());
        let account_id = self.account_id_builder.with_seed(seed)?;
        let account_code =
            str_to_account_code(&self.code).map_err(AccountBuilderError::AccountError)?;
        Ok(Account::new(account_id, vault, storage, account_code, self.nonce))
    }

    /// Build an account using the provided `seed` and `storage`.
    ///
    /// The storage items added to this builder will added on top of `storage`.
    pub fn with_seed_and_storage(
        mut self,
        seed: Word,
        mut storage: AccountStorage,
    ) -> Result<Account, AccountBuilderError> {
        let vault = AssetVault::new(&self.assets).map_err(AccountBuilderError::AssetVaultError)?;
        let inner_storage = self.storage_builder.build();

        for (key, value) in inner_storage.slots().leaves() {
            if key != 255 {
                // don't copy the reserved key
                storage.set_item(key as u8, *value).map_err(AccountBuilderError::AccountError)?;
            }
        }

        self.account_id_builder.code(&self.code);
        self.account_id_builder.storage_root(storage.root());
        let account_id = self.account_id_builder.with_seed(seed)?;
        let account_code =
            str_to_account_code(&self.code).map_err(AccountBuilderError::AccountError)?;
        Ok(Account::new(account_id, vault, storage, account_code, self.nonce))
    }
}
