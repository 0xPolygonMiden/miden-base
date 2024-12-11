use alloc::{boxed::Box, string::ToString, vec::Vec};
use core::fmt;

use vm_core::{FieldElement, WORD_SIZE};

use super::{AccountIdPrefix, AccountType, Asset, AssetError, Felt, Hasher, Word};
use crate::{
    accounts::AccountId,
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest,
};

/// Position of the faucet_id inside the [`NonFungibleAsset`] word.
const FAUCET_ID_POS: usize = 1;

// NON-FUNGIBLE ASSET
// ================================================================================================

/// A commitment to a non-fungible asset.
///
/// The commitment is constructed as follows:
///
/// - Hash the asset data producing `[hash0, hash1, hash2, hash3]`.
/// - Replace the value of `hash1` with the first felt of the faucet id (`faucet_id_hi`) producing
///   `[hash0, faucet_id_hi, hash2, hash3]`.
/// - Set the bit position [`AccountId::IS_FAUCET_MASK`] of `hash3` to be `0`. This is done to make
///   assets distinguishable from their word layout. Fungible assets will have this bit set to `1`
///   while non-fungible assets will have it set to `0`.
///
/// [`NonFungibleAsset`] itself does not contain the actual asset data. The container for this data
/// is [`NonFungibleAssetDetails`].
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
    pub fn from_parts(faucet_id: AccountIdPrefix, mut data_hash: Word) -> Result<Self, AssetError> {
        if !matches!(faucet_id.account_type(), AccountType::NonFungibleFaucet) {
            return Err(AssetError::NonFungibleFaucetIdTypeMismatch(faucet_id));
        }
        data_hash[FAUCET_ID_POS] = faucet_id.into();

        // Forces the bit at position `AccountId::IS_FAUCET_MASK` to `0`.
        //
        // Explanation of the bit flip:
        //
        // - We need to be able to determine the type of an asset by looking at some part of the
        //   word layout.
        // - Fungible assets have the first felt of an account id at word index 3. This means the
        //   bit at the mask position of fungible_asset_word[3] is always 1.
        // - Non-fungible assets have a data hash at word index 3 and we can set the bit at the mask
        //   position to 0 explicitly.
        // - This is necessary because non-fungible assets have the account id prefix at another
        //   index.
        // - This means that when looking at the bit at the mask position of an asset, fungible
        //   assets will have a 1 bit and non-fungible assets will have a 0 bit.
        //
        // This is done as an optimization, since the field element at position `3` is used as the
        // index when storing the assets into the asset vault. This strategy forces fungible
        // assets to be assigned to the same slot because it uses the faucet's account id,
        // and allows for easy merging of fungible assets. At the same time, it spreads the
        // non-fungible assets evenly across the vault, because in this case the element is
        // the result of a cryptographic hash function.

        let element3 = data_hash[3].as_int();
        data_hash[3] = Felt::new((element3 & AccountId::IS_FAUCET_MASK) ^ element3);

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
    pub fn faucet_id(&self) -> AccountIdPrefix {
        AccountIdPrefix::new_unchecked(self.0[FAUCET_ID_POS])
    }

    // HELPER FUNCTIONS
    // --------------------------------------------------------------------------------------------

    /// Validates this non-fungible asset.
    /// # Errors
    /// Returns an error if:
    /// - The faucet_id is not a valid non-fungible faucet ID.
    /// - The most significant bit of the asset is not ZERO.
    fn validate(&self) -> Result<(), AssetError> {
        let faucet_id = AccountIdPrefix::try_from(self.0[FAUCET_ID_POS])
            .map_err(|err| AssetError::InvalidFaucetAccountId(Box::new(err)))?;

        let account_type = faucet_id.account_type();
        if !matches!(account_type, AccountType::NonFungibleFaucet) {
            return Err(AssetError::NonFungibleFaucetIdTypeMismatch(faucet_id));
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
        let faucet_id_prefix: AccountIdPrefix = source.read()?;

        let hash_0: Felt = source.read()?;
        let hash_2: Felt = source.read()?;
        let hash_3: Felt = source.read()?;

        // The second felt in the data_hash will be replaced by the faucet id, so we can set it to
        // zero here.
        NonFungibleAsset::from_parts(faucet_id_prefix, [hash_0, Felt::ZERO, hash_2, hash_3])
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
    faucet_id: AccountIdPrefix,
    asset_data: Vec<u8>,
}

impl NonFungibleAssetDetails {
    /// Returns asset details instantiated from the specified faucet ID and asset data.
    ///
    /// # Errors
    /// Returns an error if the provided faucet ID is not for a non-fungible asset faucet.
    pub fn new(faucet_id: AccountIdPrefix, asset_data: Vec<u8>) -> Result<Self, AssetError> {
        if !matches!(faucet_id.account_type(), AccountType::NonFungibleFaucet) {
            return Err(AssetError::NonFungibleFaucetIdTypeMismatch(faucet_id));
        }

        Ok(Self { faucet_id, asset_data })
    }

    /// Returns ID of the faucet which issued this asset.
    pub fn faucet_id(&self) -> AccountIdPrefix {
        self.faucet_id
    }

    /// Returns asset data in binary format.
    pub fn asset_data(&self) -> &[u8] {
        &self.asset_data
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;

    use super::*;
    use crate::accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1,
        },
        AccountId,
    };

    #[test]
    fn test_non_fungible_asset_serde() {
        for non_fungible_account_id in [
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1,
        ] {
            let account_id = AccountId::try_from(non_fungible_account_id).unwrap();
            let details = NonFungibleAssetDetails::new(account_id.prefix(), vec![1, 2, 3]).unwrap();
            let non_fungible_asset = NonFungibleAsset::new(&details).unwrap();
            assert_eq!(
                non_fungible_asset,
                NonFungibleAsset::read_from_bytes(&non_fungible_asset.to_bytes()).unwrap()
            );
        }

        let account = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN).unwrap();
        let details = NonFungibleAssetDetails::new(account.prefix(), vec![4, 5, 6, 7]).unwrap();
        let asset = NonFungibleAsset::new(&details).unwrap();
        let mut asset_bytes = asset.to_bytes();

        let fungible_faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN).unwrap();

        // Set invalid Faucet ID Prefix.
        asset_bytes[0..8].copy_from_slice(&fungible_faucet_id.prefix().to_bytes());

        let err = NonFungibleAsset::read_from_bytes(&asset_bytes).unwrap_err();
        assert_matches!(err, DeserializationError::InvalidValue(msg) if msg.contains("must be of type NonFungibleFaucet"));
    }
}
