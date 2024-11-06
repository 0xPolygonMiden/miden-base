use alloc::string::ToString;
use core::fmt;

use vm_core::{
    utils::{ByteReader, ByteWriter, Deserializable, Serializable},
    FieldElement,
};
use vm_processor::DeserializationError;

use super::{
    is_not_a_non_fungible_asset, AccountId, AccountType, Asset, AssetError, Felt, Word, ZERO,
};

// FUNGIBLE ASSET
// ================================================================================================
/// A fungible asset.
///
/// A fungible asset consists of a faucet ID of the faucet which issued the asset as well as the
/// asset amount. Asset amount is guaranteed to be 2^63 - 1 or smaller.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FungibleAsset {
    faucet_id: AccountId,
    amount: u64,
}

impl FungibleAsset {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------
    /// Specifies a maximum amount value for fungible assets which can be at most a 63-bit value.
    pub const MAX_AMOUNT: u64 = (1_u64 << 63) - 1;

    /// The serialized size of a [`FungibleAsset`] in bytes.
    ///
    /// Currently an account id (felt) plus an amount (u64).
    pub const SERIALIZED_SIZE: usize = Felt::ELEMENT_BYTES + core::mem::size_of::<u64>();

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a fungible asset instantiated with the provided faucet ID and amount.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The faucet_id is not a valid fungible faucet ID.
    /// - The provided amount is greater than 2^63 - 1.
    pub const fn new(faucet_id: AccountId, amount: u64) -> Result<Self, AssetError> {
        let asset = Self { faucet_id, amount };
        asset.validate()
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
            return Err(AssetError::InconsistentFaucetIds(self.faucet_id, other.faucet_id));
        }

        let amount = self.amount.checked_add(other.amount).expect("overflow!");
        if amount > Self::MAX_AMOUNT {
            return Err(AssetError::AmountTooBig(amount));
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
    const fn validate(self) -> Result<Self, AssetError> {
        let account_type = self.faucet_id.account_type();
        if !matches!(account_type, AccountType::FungibleFaucet) {
            return Err(AssetError::NotAFungibleFaucetId(self.faucet_id, account_type));
        }

        if self.amount > Self::MAX_AMOUNT {
            return Err(AssetError::AmountTooBig(self.amount));
        }

        Ok(self)
    }
}

impl From<FungibleAsset> for Word {
    fn from(asset: FungibleAsset) -> Self {
        let mut result = Word::default();
        result[0] = Felt::new(asset.amount);
        result[3] = asset.faucet_id.into();
        debug_assert!(is_not_a_non_fungible_asset(result));
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
        if (value[1], value[2]) != (ZERO, ZERO) {
            return Err(AssetError::FungibleAssetInvalidWord(value));
        }
        let faucet_id = AccountId::try_from(value[3])
            .map_err(|e| AssetError::InvalidAccountId(e.to_string()))?;
        let amount = value[0].as_int();
        Self::new(faucet_id, amount)
    }
}

impl fmt::Display for FungibleAsset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for FungibleAsset {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        // All assets should serialize their faucet ID at the first position to allow them to be
        // easily distinguishable during deserialization.
        target.write(self.faucet_id);
        target.write(self.amount);
    }

    fn get_size_hint(&self) -> usize {
        self.faucet_id.get_size_hint() + self.amount.get_size_hint()
    }
}

impl Deserializable for FungibleAsset {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let faucet_id: AccountId = source.read()?;
        FungibleAsset::deserialize_with_account_id(faucet_id, source)
    }
}

impl FungibleAsset {
    /// Deserializes a [`FungibleAsset`] from an [`AccountId`] and the remaining data from the given
    /// `source`.
    pub(super) fn deserialize_with_account_id<R: ByteReader>(
        faucet_id: AccountId,
        source: &mut R,
    ) -> Result<Self, DeserializationError> {
        let amount: u64 = source.read()?;
        FungibleAsset::new(faucet_id, amount)
            .map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::account_id::testing::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
    };

    #[test]
    fn test_fungible_asset_serde() {
        for fungible_account_id in [
            ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3,
        ] {
            let account_id = AccountId::try_from(fungible_account_id).unwrap();
            let fungible_asset = FungibleAsset::new(account_id, 10).unwrap();
            assert_eq!(
                fungible_asset,
                FungibleAsset::read_from_bytes(&fungible_asset.to_bytes()).unwrap()
            );
        }

        let account_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3).unwrap();
        let asset = FungibleAsset::new(account_id, 50).unwrap();
        let mut asset_bytes = asset.to_bytes();
        // Set invalid Faucet ID.
        asset_bytes[0..8].copy_from_slice(&ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN.to_le_bytes());
        let err = FungibleAsset::read_from_bytes(&asset_bytes).unwrap_err();
        assert!(matches!(err, DeserializationError::InvalidValue(_)));
    }
}
