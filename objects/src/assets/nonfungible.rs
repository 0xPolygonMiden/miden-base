use alloc::{string::ToString, vec::Vec};
use core::fmt;

use vm_core::{FieldElement, WORD_SIZE};

use super::{AccountId, AccountType, Asset, AssetError, Felt, Hasher, Word, ACCOUNT_ISFAUCET_MASK};
use crate::{
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest,
};

/// Position of the faucet_id inside the [NonFungibleAsset] word.
const FAUCET_ID_POS: usize = 1;

// NON-FUNGIBLE ASSET
// ================================================================================================
/// A commitment to a non-fungible asset.
///
/// The commitment is constructed as follows:
///
/// - Hash the asset data producing `[d0, d1, d2, d3]`.
/// - Replace the value of `d1` with the faucet id producing `[d0, faucet_id, d2, d3]`.
/// - Force the bit position [ACCOUNT_ISFAUCET_MASK] of `d3` to be `0`.
///
/// [NonFungibleAsset] itself does not contain the actual asset data. The container for this data
/// [NonFungibleAssetDetails] struct.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct NonFungibleAsset(Word);

impl PartialOrd for NonFungibleAsset {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NonFungibleAsset {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        Digest::from(self.0).cmp(&Digest::from(other.0))
    }
}

impl NonFungibleAsset {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// The serialized size of a [`NonFungibleAsset`] in bytes.
    ///
    /// Currently represented as a word.
    pub const SERIALIZED_SIZE: usize = Felt::ELEMENT_BYTES * WORD_SIZE;

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
            return Err(AssetError::NotANonFungibleFaucetId(faucet_id));
        }
        data_hash[FAUCET_ID_POS] = faucet_id.into();

        // Forces the bit at position `ACCOUNT_ISFAUCET_MASK` to `0`.
        //
        // Explanation of the bit flip:
        //
        // - assets require a faucet account, the id of such accounts always has the bit at the mask
        //   position.
        // - fungible assets have the account id at position `3`, meaning the 3rd bit is always of
        //   the element at the 3rd position is always 1.
        // - non-fungible assets, have the account id at position `FAUCET_ID_POS`, so the bit at
        //   position `3` can be used to identify fungible vs. non-fungible assets
        //
        // This is done as an optimization, since the field element at position `3` is used as index
        // when storing the assets into the asset vault. This strategy forces fungible assets to be
        // assigned to the same slot because it uses the faucet's account id, and allows for easy
        // merging of fungible faucets. At the same time, it spreads the non-fungible assets evenly
        // across the vault, because in this case the element is the result of a cryptographic hash
        // function.
        let d3 = data_hash[3].as_int();
        data_hash[3] = Felt::new((d3 & ACCOUNT_ISFAUCET_MASK) ^ d3);

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

        let account_type = faucet_id.account_type();
        if !matches!(account_type, AccountType::NonFungibleFaucet) {
            return Err(AssetError::NotAFungibleFaucetId(faucet_id, account_type));
        }

        Ok(())
    }
}

impl From<NonFungibleAsset> for Word {
    fn from(asset: NonFungibleAsset) -> Self {
        asset.0
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

impl fmt::Display for NonFungibleAsset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NonFungibleAsset {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        // All assets should serialize their faucet ID at the first position to allow them to be
        // easily distinguishable during deserialization.
        target.write(self.0[FAUCET_ID_POS]);
        target.write(self.0[0]);
        target.write(self.0[2]);
        target.write(self.0[3]);
    }

    fn get_size_hint(&self) -> usize {
        Self::SERIALIZED_SIZE
    }
}

impl Deserializable for NonFungibleAsset {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let value: Word = source.read()?;
        Self::try_from(value).map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

impl NonFungibleAsset {
    /// Deserializes a [`NonFungibleAsset`] from an [`AccountId`] and the remaining data from the
    /// given `source`.
    pub(super) fn deserialize_with_account_id<R: ByteReader>(
        faucet_id: AccountId,
        source: &mut R,
    ) -> Result<Self, DeserializationError> {
        let hash_0: Felt = source.read()?;
        let hash_2: Felt = source.read()?;
        let hash_3: Felt = source.read()?;

        NonFungibleAsset::from_parts(faucet_id, [hash_0, Felt::ZERO, hash_2, hash_3])
            .map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// NON-FUNGIBLE ASSET DETAILS
// ================================================================================================

/// Details about a non-fungible asset.
///
/// Unlike [NonFungibleAsset] struct, this struct contains full details of a non-fungible asset.
#[derive(Debug, Clone, PartialEq, Eq)]
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
            return Err(AssetError::NotANonFungibleFaucetId(faucet_id));
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
