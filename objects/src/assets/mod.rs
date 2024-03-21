use alloc::string::ToString;

use super::{
    accounts::{AccountId, AccountType},
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
///   Element 1 will be:
///    - ZERO for a fungible asset
///    - non-ZERO for a non-fungible asset
///   The most significant bit will be:
///    - ONE for a fungible asset
///    - ZERO for a non-fungible asset
///
/// The above properties guarantee that there can never be a collision between a fungible and a
/// non-fungible asset.
///
/// The methodology for constructing fungible and non-fungible assets is described below.
///
/// # Fungible assets
/// The most significant element of a fungible asset is set to the ID of the faucet which issued
/// the asset. This guarantees the properties described above (the first bit is ONE).
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
/// - Lastly, the most significant bit of d3 is set to ZERO.
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
        let first_bit = value[3].as_int() >> 63;
        match first_bit {
            0 => Asset::NonFungible(unsafe { NonFungibleAsset::new_unchecked(value) }),
            1 => Asset::Fungible(FungibleAsset::new_unchecked(value)),
            _ => unreachable!(),
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

    /// Returns the key which is used to store this asset in the account vault.
    pub fn vault_key(&self) -> Word {
        match self {
            Self::Fungible(asset) => asset.vault_key(),
            Self::NonFungible(asset) => asset.vault_key(),
        }
    }
}

impl From<Asset> for Word {
    fn from(asset: Asset) -> Self {
        use Asset::*;
        match asset {
            Fungible(asset) => asset.into(),
            NonFungible(asset) => asset.into(),
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
        use Asset::*;
        match asset {
            Fungible(asset) => asset.into(),
            NonFungible(asset) => asset.into(),
        }
    }
}

impl From<&Asset> for [u8; 32] {
    fn from(value: &Asset) -> Self {
        (*value).into()
    }
}

impl TryFrom<Word> for Asset {
    type Error = AssetError;

    fn try_from(value: Word) -> Result<Self, Self::Error> {
        let first_bit = value[3].as_int() >> 63;
        match first_bit {
            0 => NonFungibleAsset::try_from(value).map(Asset::from),
            1 => FungibleAsset::try_from(value).map(Asset::from),
            _ => unreachable!(),
        }
    }
}

impl TryFrom<[u8; 32]> for Asset {
    type Error = AssetError;

    fn try_from(value: [u8; 32]) -> Result<Self, Self::Error> {
        let first_bit = value[31] >> 7;
        match first_bit {
            0 => NonFungibleAsset::try_from(value).map(Asset::from),
            1 => FungibleAsset::try_from(value).map(Asset::from),
            _ => unreachable!(),
        }
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
            .map_err(|v| DeserializationError::InvalidValue(format!("{v:?}")))?;
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
