use alloc::{string::ToString, vec::Vec};
use core::fmt;

use super::{parse_word, AccountId, AccountType, Asset, AssetError, Felt, Hasher, Word};

/// Position of the faucet_id inside the [NonFungibleAsset] word.
const FAUCET_ID_POS: usize = 1;

// NON-FUNGIBLE ASSET
// ================================================================================================
/// A commitment to a non-fungible asset.
///
/// A non-fungible asset consists of 4 field elements which are computed by hashing asset data
/// (which can be of arbitrary length) to produce: [d0, d1, d2, d3].  We then replace d1 with the
/// faucet_id that issued the asset: [d0, faucet_id, d2, d3]. We then set the most significant bit
/// of the most significant element to ZERO.
///
/// [NonFungibleAsset] itself does not contain the actual asset data. The container for this data
/// [NonFungibleAssetDetails] struct.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct NonFungibleAsset(Word);

impl NonFungibleAsset {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a non-fungible asset created from the specified asset details.
    ///
    /// # Errors
    /// Returns an error if the provided faucet ID is not for a non-fungible asset faucet.
    pub fn new(details: &NonFungibleAssetDetails) -> Result<Self, AssetError> {
        let data_hash = Hasher::hash(details.asset_data());
        Self::from_parts(details.faucet_id(), data_hash.into())
    }

    /// Return a non-fungible asset created from the specified faucet and using the provided
    /// hash of the asset's data.
    ///
    /// Hash of the asset's data is expected to be computed from the binary representation of the
    /// asset's data.
    ///
    /// # Errors
    /// Returns an error if the provided faucet ID is not for a non-fungible asset faucet.
    pub fn from_parts(faucet_id: AccountId, mut data_hash: Word) -> Result<Self, AssetError> {
        if !matches!(faucet_id.account_type(), AccountType::NonFungibleFaucet) {
            return Err(AssetError::not_a_non_fungible_faucet_id(faucet_id));
        }
        // set the element 1 to the faucet_id
        data_hash[FAUCET_ID_POS] = faucet_id.into();

        // set the first bit of the asset to 0; we can do this because setting the first bit to 0
        // will always result in a valid field element.
        data_hash[3] = Felt::new((data_hash[3].as_int() << 1) >> 1);

        // construct an asset
        let asset = Self(data_hash);

        Ok(asset)
    }

    /// Creates a new [NonFungibleAsset] without checking its validity.
    ///
    /// # Safety
    /// This function required that the provided value is a valid word representation of a
    /// [NonFungibleAsset].
    pub unsafe fn new_unchecked(value: Word) -> NonFungibleAsset {
        NonFungibleAsset(value)
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------
    pub fn vault_key(&self) -> Word {
        self.0
    }

    /// Return ID of the faucet which issued this asset.
    pub fn faucet_id(&self) -> AccountId {
        AccountId::new_unchecked(self.0[FAUCET_ID_POS])
    }

    // HELPER FUNCTIONS
    // --------------------------------------------------------------------------------------------

    /// Validates this non-fungible asset.
    /// # Errors
    /// Returns an error if:
    /// - The faucet_id is not a valid non-fungible faucet ID.
    /// - The most significant bit of the asset is not ZERO.
    fn validate(&self) -> Result<(), AssetError> {
        let faucet_id = AccountId::try_from(self.0[FAUCET_ID_POS])
            .map_err(|e| AssetError::InvalidAccountId(e.to_string()))?;

        if !matches!(faucet_id.account_type(), AccountType::NonFungibleFaucet) {
            return Err(AssetError::not_a_fungible_faucet_id(faucet_id));
        }

        if self.0[3].as_int() >> 63 != 0 {
            return Err(AssetError::non_fungible_asset_invalid_first_bit());
        }

        Ok(())
    }
}

impl From<NonFungibleAsset> for Word {
    fn from(asset: NonFungibleAsset) -> Self {
        asset.0
    }
}

impl From<NonFungibleAsset> for [u8; 32] {
    fn from(asset: NonFungibleAsset) -> Self {
        let mut result = [0_u8; 32];
        result[..8].copy_from_slice(&asset.0[0].as_int().to_le_bytes());
        result[8..16].copy_from_slice(&asset.0[FAUCET_ID_POS].as_int().to_le_bytes());
        result[16..24].copy_from_slice(&asset.0[2].as_int().to_le_bytes());
        result[24..].copy_from_slice(&asset.0[3].as_int().to_le_bytes());
        result
    }
}

impl From<NonFungibleAsset> for Asset {
    fn from(asset: NonFungibleAsset) -> Self {
        Asset::NonFungible(asset)
    }
}

impl TryFrom<Word> for NonFungibleAsset {
    type Error = AssetError;

    fn try_from(value: Word) -> Result<Self, Self::Error> {
        let asset = Self(value);
        asset.validate()?;
        Ok(asset)
    }
}

impl TryFrom<[u8; 32]> for NonFungibleAsset {
    type Error = AssetError;

    fn try_from(value: [u8; 32]) -> Result<Self, Self::Error> {
        let word = parse_word(value)?;
        Self::try_from(word)
    }
}

impl fmt::Display for NonFungibleAsset {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

// NON-FUNGIBLE ASSET DETAILS
// ================================================================================================

/// Details about a non-fungible asset.
///
/// Unlike [NonFungibleAsset] struct, this struct contains full details of a non-fungible asset.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NonFungibleAssetDetails {
    faucet_id: AccountId,
    asset_data: Vec<u8>,
}

impl NonFungibleAssetDetails {
    /// Returns asset details instantiated from the specified faucet ID and asset data.
    ///
    /// # Errors
    /// Returns an error if the provided faucet ID is not for a non-fungible asset faucet.
    pub fn new(faucet_id: AccountId, asset_data: Vec<u8>) -> Result<Self, AssetError> {
        if !matches!(faucet_id.account_type(), AccountType::NonFungibleFaucet) {
            return Err(AssetError::not_a_non_fungible_faucet_id(faucet_id));
        }

        Ok(Self { faucet_id, asset_data })
    }

    /// Returns ID of the faucet which issued this asset.
    pub fn faucet_id(&self) -> AccountId {
        self.faucet_id
    }

    /// Returns asset data in binary format.
    pub fn asset_data(&self) -> &[u8] {
        &self.asset_data
    }
}
