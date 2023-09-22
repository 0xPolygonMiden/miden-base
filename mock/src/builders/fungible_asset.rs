use miden_objects::{
    accounts::{AccountId, AccountType},
    assets::FungibleAsset,
    AssetError,
};

/// Builder for an `FungibleAsset`, the builder can be configured and used multipled times.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FungibleAssetBuilder {
    faucet_id: AccountId,
    amount: u64,
}

impl FungibleAssetBuilder {
    pub const DEFAULT_AMOUNT: u64 = 10;

    pub fn new(faucet_id: AccountId) -> Result<Self, AssetError> {
        if !matches!(faucet_id.account_type(), AccountType::FungibleFaucet) {
            return Err(AssetError::not_a_fungible_faucet_id(faucet_id));
        }

        Ok(Self {
            faucet_id,
            amount: Self::DEFAULT_AMOUNT,
        })
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
