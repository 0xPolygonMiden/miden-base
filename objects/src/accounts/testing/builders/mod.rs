use core::fmt::Display;

use crate::{crypto::merkle::MerkleError, AccountError, AssetVaultError};

pub mod account;
pub mod account_id;

#[derive(Debug)]
pub enum AccountBuilderError {
    AccountError(AccountError),
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
