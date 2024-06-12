use alloc::string::ToString;

use super::{
    accounts::{AccountId, AccountType, ACCOUNT_ISFAUCET_MASK},
    utils::serde::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    AssetError, Felt, Hasher, Word, ZERO,
};

mod fungible;
pub use fungible::FungibleAsset;

mod nonfungible;
pub use nonfungible::{NonFungibleAsset, NonFungibleAssetDetails};

mod token_symbol;
pub use token_symbol::TokenSymbol;

mod vault;
pub use vault::AssetVault;

// ASSET
// ================================================================================================

/// A fungible or a non-fungible asset.
///
/// All assets are encoded using a single word (4 elements) such that it is easy to determine the
/// type of an asset both inside and outside Miden VM. Specifically:
/// - Element 1 will be:
///   - ZERO for a fungible asset
///   - non-ZERO for a non-fungible asset
/// - The 3rd most significant bit will be:
///   - 1 for a fungible asset
///   - 0 for a non-fungible asset
///
/// The above properties guarantee that there can never be a collision between a fungible and a
/// non-fungible asset.
///
/// The methodology for constructing fungible and non-fungible assets is described below.
///
/// # Fungible assets
/// The most significant element of a fungible asset is set to the ID of the faucet which issued
/// the asset. This guarantees the properties described above (the 3rd most significant bit is ONE).
///
/// The least significant element is set to the amount of the asset. This amount cannot be greater
/// than 2^63 - 1 and thus requires 63-bits to store.
///
/// Elements 1 and 2 are set to ZERO.
///
/// It is impossible to find a collision between two fungible assets issued by different faucets as
/// the faucet_id is included in the description of the asset and this is guaranteed to be different
/// for each faucet as per the faucet creation logic.
///
/// # Non-fungible assets
/// The 4 elements of non-fungible assets are computed as follows:
/// - First the asset data is hashed. This compresses an asset of an arbitrary length to 4 field
///   elements: [d0, d1, d2, d3].
/// - d1 is then replaced with the faucet_id which issues the asset: [d0, faucet_id, d2, d3].
/// - Lastly, the 3rd most significant bit of d3 is set to ZERO.
///
/// It is impossible to find a collision between two non-fungible assets issued by different faucets
/// as the faucet_id is included in the description of the non-fungible asset and this is guaranteed
/// to be different as per the faucet creation logic. Collision resistance for non-fungible assets
/// issued by the same faucet is ~2^95.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Asset {
    Fungible(FungibleAsset),
    NonFungible(NonFungibleAsset),
}

impl Asset {
    /// Creates a new [Asset] without checking its validity.
    pub(crate) fn new_unchecked(value: Word) -> Asset {
        if is_not_a_non_fungible_asset(value) {
            Asset::Fungible(FungibleAsset::new_unchecked(value))
        } else {
            Asset::NonFungible(unsafe { NonFungibleAsset::new_unchecked(value) })
        }
    }

    /// Returns true if this asset is the same as the specified asset.
    ///
    /// Two assets are defined to be the same if:
    /// - For fungible assets, if they were issued by the same faucet.
    /// - For non-fungible assets, if the assets are identical.
    pub fn is_same(&self, other: &Self) -> bool {
        use Asset::*;
        match (self, other) {
            (Fungible(l), Fungible(r)) => l.is_from_same_faucet(r),
            (NonFungible(l), NonFungible(r)) => l == r,
            _ => false,
        }
    }

    /// Returns true if this asset is a fungible asset.
    pub const fn is_fungible(&self) -> bool {
        matches!(self, Self::Fungible(_))
    }

    /// Returns ID of the faucet which issued this asset.
    pub fn faucet_id(&self) -> AccountId {
        match self {
            Self::Fungible(asset) => asset.faucet_id(),
            Self::NonFungible(asset) => asset.faucet_id(),
        }
    }

    /// Returns the key which is used to store this asset in the account vault.
    pub fn vault_key(&self) -> Word {
        match self {
            Self::Fungible(asset) => asset.vault_key(),
            Self::NonFungible(asset) => asset.vault_key(),
        }
    }

    /// Returns the inner fungible asset, or panics if the asset is not fungible.
    pub fn unwrap_fungible(&self) -> FungibleAsset {
        match self {
            Asset::Fungible(asset) => *asset,
            Asset::NonFungible(_) => panic!("the asset is non-fungible"),
        }
    }

    /// Returns the inner non-fungible asset, or panics if the asset is fungible.
    pub fn unwrap_non_fungible(&mut self) -> NonFungibleAsset {
        match self {
            Asset::Fungible(_) => panic!("the asset is fungible"),
            Asset::NonFungible(asset) => *asset,
        }
    }
}

impl From<Asset> for Word {
    fn from(asset: Asset) -> Self {
        match asset {
            Asset::Fungible(asset) => asset.into(),
            Asset::NonFungible(asset) => asset.into(),
        }
    }
}

impl From<&Asset> for Word {
    fn from(value: &Asset) -> Self {
        (*value).into()
    }
}

impl From<Asset> for [u8; 32] {
    fn from(asset: Asset) -> Self {
        match asset {
            Asset::Fungible(asset) => asset.into(),
            Asset::NonFungible(asset) => asset.into(),
        }
    }
}

impl From<&Asset> for [u8; 32] {
    fn from(value: &Asset) -> Self {
        (*value).into()
    }
}

impl TryFrom<&Word> for Asset {
    type Error = AssetError;

    fn try_from(value: &Word) -> Result<Self, Self::Error> {
        (*value).try_into()
    }
}

impl TryFrom<Word> for Asset {
    type Error = AssetError;

    fn try_from(value: Word) -> Result<Self, Self::Error> {
        if is_not_a_non_fungible_asset(value) {
            FungibleAsset::try_from(value).map(Asset::from)
        } else {
            NonFungibleAsset::try_from(value).map(Asset::from)
        }
    }
}

impl TryFrom<[u8; 32]> for Asset {
    type Error = AssetError;

    fn try_from(value: [u8; 32]) -> Result<Self, Self::Error> {
        parse_word(value)?.try_into()
    }
}

impl TryFrom<&[u8; 32]> for Asset {
    type Error = AssetError;

    fn try_from(value: &[u8; 32]) -> Result<Self, Self::Error> {
        (*value).try_into()
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for Asset {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let data: [u8; 32] = self.into();
        target.write_bytes(&data);
    }
}

impl Deserializable for Asset {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let data_vec = source.read_vec(32)?;
        let data_array: [u8; 32] = data_vec.try_into().expect("Vec must be of size 32");

        let asset = Asset::try_from(&data_array)
            .map_err(|error| DeserializationError::InvalidValue(format!("{error}")))?;
        Ok(asset)
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn parse_word(bytes: [u8; 32]) -> Result<Word, AssetError> {
    Ok([
        parse_felt(&bytes[..8])?,
        parse_felt(&bytes[8..16])?,
        parse_felt(&bytes[16..24])?,
        parse_felt(&bytes[24..])?,
    ])
}

fn parse_felt(bytes: &[u8]) -> Result<Felt, AssetError> {
    Felt::try_from(bytes).map_err(|err| AssetError::invalid_field_element(err.to_string()))
}

/// Returns `true` if asset in [Word] is not a non-fungible asset.
///
/// Note: this does not mean that the word is a fungible asset as the word may contain an value
/// which is not a valid asset.
fn is_not_a_non_fungible_asset(asset: Word) -> bool {
    // For fungible assets, the position `3` contains the faucet's account id, in which case the
    // bit is set. For non-fungible assets have the bit always set to `0`.
    (asset[3].as_int() & ACCOUNT_ISFAUCET_MASK) == ACCOUNT_ISFAUCET_MASK
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use miden_crypto::{
        utils::{Deserializable, Serializable},
        Word,
    };

    use super::{Asset, FungibleAsset, NonFungibleAsset, NonFungibleAssetDetails};
    use crate::accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1,
        },
        AccountId,
    };

    #[test]
    fn test_asset_serde() {
        for fungible_account_id in [
            ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3,
        ] {
            let account_id = AccountId::try_from(fungible_account_id).unwrap();
            let fungible_asset: Asset = FungibleAsset::new(account_id, 10).unwrap().into();
            assert_eq!(fungible_asset, Asset::read_from_bytes(&fungible_asset.to_bytes()).unwrap());
        }

        for non_fungible_account_id in [
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1,
        ] {
            let account_id = AccountId::try_from(non_fungible_account_id).unwrap();
            let details = NonFungibleAssetDetails::new(account_id, vec![1, 2, 3]).unwrap();
            let non_fungible_asset: Asset = NonFungibleAsset::new(&details).unwrap().into();
            assert_eq!(
                non_fungible_asset,
                Asset::read_from_bytes(&non_fungible_asset.to_bytes()).unwrap()
            );
        }
    }

    #[test]
    fn test_new_unchecked() {
        for fungible_account_id in [
            ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3,
        ] {
            let account_id = AccountId::try_from(fungible_account_id).unwrap();
            let fungible_asset: Asset = FungibleAsset::new(account_id, 10).unwrap().into();
            assert_eq!(fungible_asset, Asset::new_unchecked(Word::from(&fungible_asset)));
        }

        for non_fungible_account_id in [
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1,
        ] {
            let account_id = AccountId::try_from(non_fungible_account_id).unwrap();
            let details = NonFungibleAssetDetails::new(account_id, vec![1, 2, 3]).unwrap();
            let non_fungible_asset: Asset = NonFungibleAsset::new(&details).unwrap().into();
            assert_eq!(non_fungible_asset, Asset::new_unchecked(Word::from(non_fungible_asset)));
        }
    }
}
