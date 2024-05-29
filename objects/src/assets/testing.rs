use rand::{distributions::Standard, Rng};

use super::FungibleAsset;
use crate::{
    accounts::{AccountId, AccountType},
    assets::{NonFungibleAsset, NonFungibleAssetDetails},
    AssetError,
};

/// Builder for an `NonFungibleAssetDetails`, the builder can be configured and used multiplied times.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NonFungibleAssetDetailsBuilder<T> {
    faucet_id: AccountId,
    rng: T,
}

/// Builder for an `FungibleAsset`, the builder can be configured and used multiplied times.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FungibleAssetBuilder {
    faucet_id: AccountId,
    amount: u64,
}

impl<T: Rng> NonFungibleAssetDetailsBuilder<T> {
    pub fn new(faucet_id: AccountId, rng: T) -> Result<Self, AssetError> {
        if !matches!(faucet_id.account_type(), AccountType::NonFungibleFaucet) {
            return Err(AssetError::not_a_non_fungible_faucet_id(faucet_id));
        }

        Ok(Self { faucet_id, rng })
    }

    pub fn build(&mut self) -> Result<NonFungibleAssetDetails, AssetError> {
        let data = (&mut self.rng).sample_iter(Standard).take(5).collect();
        NonFungibleAssetDetails::new(self.faucet_id, data)
    }
}

/// Builder for an `NonFungibleAsset`, the builder can be configured and used multiplied times.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NonFungibleAssetBuilder<T> {
    details_builder: NonFungibleAssetDetailsBuilder<T>,
}

impl<T: Rng> NonFungibleAssetBuilder<T> {
    pub fn new(faucet_id: AccountId, rng: T) -> Result<Self, AssetError> {
        let details_builder = NonFungibleAssetDetailsBuilder::new(faucet_id, rng)?;
        Ok(Self { details_builder })
    }

    pub fn build(&mut self) -> Result<NonFungibleAsset, AssetError> {
        let details = self.details_builder.build()?;
        NonFungibleAsset::new(&details)
    }
}

impl FungibleAssetBuilder {
    pub const DEFAULT_AMOUNT: u64 = 10;

    pub fn new(faucet_id: AccountId) -> Result<Self, AssetError> {
        let account_type = faucet_id.account_type();
        if !matches!(account_type, AccountType::FungibleFaucet) {
            return Err(AssetError::not_a_fungible_faucet_id(faucet_id, account_type));
        }

        Ok(Self { faucet_id, amount: Self::DEFAULT_AMOUNT })
    }

    pub fn amount(&mut self, amount: u64) -> Result<&mut Self, AssetError> {
        if amount > FungibleAsset::MAX_AMOUNT {
            return Err(AssetError::amount_too_big(amount));
        }

        self.amount = amount;
        Ok(self)
    }

    pub fn with_amount(&self, amount: u64) -> Result<FungibleAsset, AssetError> {
        FungibleAsset::new(self.faucet_id, amount)
    }

    pub fn build(&self) -> Result<FungibleAsset, AssetError> {
        FungibleAsset::new(self.faucet_id, self.amount)
    }
}
