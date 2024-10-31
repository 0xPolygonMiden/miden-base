use alloc::vec::Vec;
use core::fmt::Display;

use miden_crypto::merkle::MerkleError;
use rand::Rng;

use super::account_id::AccountIdBuilder;
use crate::{
    accounts::{Account, AccountComponent, AccountStorageMode, AccountType},
    assets::{Asset, AssetVault},
    AccountError, AssetVaultError, Felt, Word, ZERO,
};

/// Builder for an [`Account`].
/// The builder allows for a fluent API to construct an account. Each account needs a unique
/// builder.
#[derive(Clone)]
pub struct AccountBuilder<T> {
    assets: Vec<Asset>,
    components: Vec<AccountComponent>,
    nonce: Felt,
    account_id_builder: AccountIdBuilder<T>,
}

impl<T: Rng> AccountBuilder<T> {
    pub fn new(rng: T) -> Self {
        Self {
            assets: vec![],
            components: vec![],
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

    pub fn add_component(mut self, account_component: impl Into<AccountComponent>) -> Self {
        self.components.push(account_component.into());
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

    pub fn storage_mode(mut self, storage_mode: AccountStorageMode) -> Self {
        self.account_id_builder.storage_mode(storage_mode);
        self
    }

    pub fn build(mut self) -> Result<(Account, Word), AccountBuilderError> {
        let vault = AssetVault::new(&self.assets).map_err(AccountBuilderError::AssetVaultError)?;

        let account_type = self.account_id_builder.get_account_type();
        let (code, storage) = Account::initialize_from_components(account_type, &self.components)
            .map_err(AccountBuilderError::AccountError)?;

        self.account_id_builder.code_commitment(code.commitment());
        self.account_id_builder.storage_commitment(storage.commitment());
        let (account_id, seed) = self.account_id_builder.build()?;

        let account = Account::from_parts(account_id, vault, storage, code, self.nonce);
        Ok((account, seed))
    }
}

#[derive(Debug)]
pub enum AccountBuilderError {
    AccountError(AccountError),
    AccountCodeNotSet,
    AssetVaultError(AssetVaultError),
    MerkleError(MerkleError),

    /// When the created [AccountId] doesn't match the builder's configured [AccountType].
    SeedAndAccountTypeMismatch,

    /// When the created [AccountId] doesn't match the builder's `on_chain` config.
    SeedAndOnChainMismatch,
}

impl Display for AccountBuilderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AccountBuilderError {}
