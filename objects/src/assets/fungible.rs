use alloc::string::ToString;
use core::fmt;

use super::{parse_word, AccountId, AccountType, Asset, AssetError, Felt, Word, ZERO};

// FUNGIBLE ASSET
// ================================================================================================
/// A fungible asset.
///
/// A fungible asset consists of a faucet ID of the faucet which issued the asset as well as the
/// asset amount. Asset amount is guaranteed to be 2^63 - 1 or smaller.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FungibleAsset {
    faucet_id: AccountId,
    amount: u64,
}

impl FungibleAsset {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------
    /// Specifies a maximum amount value for fungible assets which can be at most a 63-bit value.
    pub const MAX_AMOUNT: u64 = (1_u64 << 63) - 1;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a fungible asset instantiated with the provided faucet ID and amount.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The faucet_id is not a valid fungible faucet ID.
    /// - The provided amount is greater than 2^63 - 1.
    pub fn new(faucet_id: AccountId, amount: u64) -> Result<Self, AssetError> {
        // construct the asset and make sure it passes the validation logic
        let asset = Self { faucet_id, amount };

        // validate fungible asset
        asset.validate()?;

        Ok(asset)
    }

    /// Creates a new [FungibleAsset] without checking its validity.
    pub(crate) fn new_unchecked(value: Word) -> FungibleAsset {
        FungibleAsset {
            faucet_id: AccountId::new_unchecked(value[3]),
            amount: value[0].as_int(),
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Return ID of the faucet which issued this asset.
    pub fn faucet_id(&self) -> AccountId {
        self.faucet_id
    }

    /// Returns the amount of this asset.
    pub fn amount(&self) -> u64 {
        self.amount
    }

    /// Returns true if this and the other assets were issued from the same faucet.
    pub fn is_from_same_faucet(&self, other: &Self) -> bool {
        self.faucet_id == other.faucet_id
    }

    /// Returns the key which is used to store this asset in the account vault.
    pub fn vault_key(&self) -> Word {
        let mut key = Word::default();
        key[3] = self.faucet_id.into();
        key
    }

    // OPERATIONS
    // --------------------------------------------------------------------------------------------

    /// Adds two fungible assets together and returns the result.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The assets were not issued by the same faucet.
    /// - The total value of assets is greater than or equal to 2^63.
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, other: Self) -> Result<Self, AssetError> {
        if self.faucet_id != other.faucet_id {
            return Err(AssetError::inconsistent_faucet_ids(self.faucet_id, other.faucet_id));
        }

        let amount = self.amount.checked_add(other.amount).expect("overflow!");
        if amount > Self::MAX_AMOUNT {
            return Err(AssetError::amount_too_big(amount));
        }

        Ok(Self { faucet_id: self.faucet_id, amount })
    }

    /// Subtracts the specified amount from this asset and returns the resulting asset.
    ///
    /// # Errors
    /// Returns an error if this asset's amount is smaller than the requested amount.
    pub fn sub(&mut self, amount: u64) -> Result<Self, AssetError> {
        self.amount = self
            .amount
            .checked_sub(amount)
            .ok_or(AssetError::AssetAmountNotSufficient(self.amount, amount))?;

        Ok(FungibleAsset { faucet_id: self.faucet_id, amount })
    }

    // HELPER FUNCTIONS
    // --------------------------------------------------------------------------------------------

    /// Validates this fungible asset.
    /// # Errors
    /// Returns an error if:
    /// - The faucet_id is not a valid fungible faucet ID.
    /// - The provided amount is greater than 2^63 - 1.
    fn validate(&self) -> Result<(), AssetError> {
        if !matches!(self.faucet_id.account_type(), AccountType::FungibleFaucet) {
            return Err(AssetError::not_a_fungible_faucet_id(self.faucet_id));
        }

        if self.amount > Self::MAX_AMOUNT {
            return Err(AssetError::amount_too_big(self.amount));
        }

        Ok(())
    }
}

impl From<FungibleAsset> for Word {
    fn from(asset: FungibleAsset) -> Self {
        let mut result = Word::default();
        result[0] = Felt::new(asset.amount);
        result[3] = asset.faucet_id.into();
        result
    }
}

impl From<FungibleAsset> for [u8; 32] {
    fn from(asset: FungibleAsset) -> Self {
        let mut result = [0_u8; 32];
        let id_bytes: [u8; 8] = asset.faucet_id.into();
        result[..8].copy_from_slice(&asset.amount.to_le_bytes());
        result[24..].copy_from_slice(&id_bytes);
        result
    }
}

impl From<FungibleAsset> for Asset {
    fn from(asset: FungibleAsset) -> Self {
        Asset::Fungible(asset)
    }
}

impl TryFrom<Word> for FungibleAsset {
    type Error = AssetError;

    fn try_from(value: Word) -> Result<Self, Self::Error> {
        // return an error if elements 1 and 2 are not zero
        if (value[1], value[2]) != (ZERO, ZERO) {
            return Err(AssetError::fungible_asset_invalid_word(value));
        }
        let faucet_id = AccountId::try_from(value[3])
            .map_err(|e| AssetError::invalid_account_id(e.to_string()))?;
        let amount = value[0].as_int();
        Self::new(faucet_id, amount)
    }
}

impl TryFrom<[u8; 32]> for FungibleAsset {
    type Error = AssetError;

    fn try_from(value: [u8; 32]) -> Result<Self, Self::Error> {
        let word = parse_word(value)?;
        Self::try_from(word)
    }
}

impl fmt::Display for FungibleAsset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
