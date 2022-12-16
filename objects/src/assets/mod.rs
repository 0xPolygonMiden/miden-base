use super::{AccountId, AssetError, Digest, Felt, Hasher, StarkField, Word, ZERO};
use core::{fmt, ops::Deref};

// ASSET
// ================================================================================================

/// A fungible or a non-fungible asset.
///
/// All assets are encoded using a single word (4 elements) such that it is easy to determine the
/// type of an asset both inside and outside Miden VM. Specifically:
/// - The first bit of a fungible asset is always ONE, and the last 32-bits of the 3rd element
///   are always set to ZEROs.
/// - The first bit of a non-fungible asset is always ZERO, and the last 32-bits of the 3rd element
///   are always set to 2^31 (i.e., ONE followed by 31 ZEROs).
///
/// The above properties guarantee that there can never be a collision between a fungible and a
/// non-fungible asset. Collision resistance of both fungible and non-fungible assets is ~110 bits.
/// However, for fungible assets collision resistance is not important as to get a collision for
/// fungible assets there must be a collision in account IDs which can be prevented at account
/// creation time.
///
/// The methodology for constructing fungible and non-fungible assets is described below.
///
/// # Fungible assets
/// The first 3 elements of a fungible asset are set to the ID of the faucet which issued the
/// asset. This guarantees the properties described above (the first bit is ONE, and the 32 least
/// significant bits of the 3rd element are all ZEROs).
///
/// The last element is set to the amount of the asset. This amount cannot be greater than
/// 2^63 - 1 and thus requires 63-bits to store.
///
/// # Non-fungible assets
/// The 4 elements of non-fungible assets are computed as follows:
/// - First the asset data is hashed. This compresses an asset of an arbitrary length to 4 field
///   elements.
/// - The result of step 1 is hashed together with the ID of the faucet which issued the asset.
///   This guarantees that finding two assets with the same hash is infeasible.
/// - Lastly, the first bit of the result is set to ZERO, and the last 32 bits of the 3rd element
///   are set to 2^31 (i.e., ONE followed by 32 ZEROs).
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Asset {
    Fungible(FungibleAsset),
    NonFungible(NonFungibleAsset),
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

impl From<Asset> for [u8; 32] {
    fn from(asset: Asset) -> Self {
        use Asset::*;
        match asset {
            Fungible(asset) => asset.into(),
            NonFungible(asset) => asset.into(),
        }
    }
}

impl TryFrom<Word> for Asset {
    type Error = AssetError;

    fn try_from(value: Word) -> Result<Self, Self::Error> {
        let first_bit = value[0].as_int() >> 63;
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
        let first_bit = value[0] >> 7;
        match first_bit {
            0 => NonFungibleAsset::try_from(value).map(Asset::from),
            1 => FungibleAsset::try_from(value).map(Asset::from),
            _ => unreachable!(),
        }
    }
}

// FUNGIBLE ASSET
// ================================================================================================
/// A fungible asset.
///
/// A fungible asset consists of a faucet ID of the faucet which issued the asset as well as the
/// asset amount. Asset amount is guaranteed to be 2^63 - 1 or smaller.
#[derive(Debug, Clone, Eq, PartialEq)]
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
    /// - The provided faucet ID is not for a fungible asset faucet.
    /// - The provide amount is greater than 2^63 - 1.
    pub fn new(faucet_id: AccountId, amount: u64) -> Result<Self, AssetError> {
        if !faucet_id.is_fungible_faucet() {
            return Err(AssetError::not_a_fungible_faucet_id(faucet_id));
        }

        // construct the asset and make sure it passes the validation logic
        let asset = Self { faucet_id, amount };
        asset.validate()?;
        Ok(asset)
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

        Ok(Self {
            faucet_id: self.faucet_id,
            amount,
        })
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

        Ok(FungibleAsset {
            faucet_id: self.faucet_id,
            amount,
        })
    }

    // HELPER FUNCTIONS
    // --------------------------------------------------------------------------------------------

    /// Returns an error if:
    /// - The first bit of the asset is not ONE.
    /// - The 32 least significant bits of the 3rd element are not set to ZEROs.
    /// - The amount is greater than or equal to 2^63.
    fn validate(&self) -> Result<(), AssetError> {
        let id_elements: &[Felt; 3] = &self.faucet_id;
        let first_bit = id_elements[0].as_int() >> 63;
        if first_bit != 1 {
            return Err(AssetError::fungible_asset_invalid_first_bit());
        }

        let tag = id_elements[3].as_int() as u32;
        if tag != AccountId::FUNGIBLE_FAUCET_TAG {
            return Err(AssetError::fungible_asset_invalid_tag(tag));
        }

        if self.amount > Self::MAX_AMOUNT {
            return Err(AssetError::amount_too_big(self.amount));
        }

        Ok(())
    }
}

impl From<FungibleAsset> for Word {
    fn from(asset: FungibleAsset) -> Self {
        let mut result: Word = asset.faucet_id.into();
        debug_assert_eq!(result[3], ZERO);
        result[3] = Felt::new(asset.amount);
        result
    }
}

impl From<FungibleAsset> for [u8; 32] {
    fn from(asset: FungibleAsset) -> Self {
        let mut result = [0_u8; 32];
        let id_bytes: [u8; 24] = asset.faucet_id.into();
        result[..24].copy_from_slice(&id_bytes);
        result[24..].copy_from_slice(&asset.amount.to_le_bytes());
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
        let faucet_id = AccountId::try_from([value[0], value[1], value[2]])
            .map_err(AssetError::invalid_account_id)?;
        let amount = value[3].as_int();
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
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

// NON-FUNGIBLE ASSET
// ================================================================================================
/// A commitment to a non-fungible asset.
///
/// A non-fungible asset consists of 4 field elements which are computed by hashing asset data
/// (which can be of arbitrary length) with the ID of the faucet which issued the asset, and then
/// setting bits in the result to ensure the the asset encoding is correct.
///
/// Specifically:
/// - The first bit of the asset is set to ZERO.
/// - The 32 least significant bits of the 3rd element are set to 2^31.
///
/// [NonFungibleAsset] itself does not contain the actual asset data. The container for this data
/// [NonFungibleAssetDetails] struct.
#[derive(Debug, Clone, Eq, PartialEq)]
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
        Self::from_parts(details.faucet_id(), data_hash)
    }

    /// Return a non-fungible asset created from the specified faucet and using the provided
    /// hash of the asset's data.
    ///
    /// Hash of the asset's data is expected to be computed from the binary representation of the
    /// asset's data.
    ///
    /// # Errors
    /// Returns an error if the provided faucet ID is not for a non-fungible asset faucet.
    pub fn from_parts(faucet_id: AccountId, data_hash: Digest) -> Result<Self, AssetError> {
        if !faucet_id.is_non_fungible_faucet() {
            return Err(AssetError::not_a_non_fungible_faucet_id(faucet_id));
        }

        // hash faucet ID and asset data hash together
        let faucet_id: Word = faucet_id.into();
        let mut asset: Word = Hasher::merge(&[faucet_id.into(), data_hash]).into();

        // set the first bit of the asset to 0; we can do this because setting the first bit to 0
        // will always result in a valid field element.
        asset[0] = Felt::new((asset[0].as_int() << 1) >> 1);

        // set the 32 least significant bits of the 3rd element to the non-fungible asset tag
        let temp = (asset[2].as_int() >> 32) << 32;
        asset[2] = Felt::new(temp | AccountId::NON_FUNGIBLE_FAUCET_TAG as u64);

        // construct an asset and make sure it passes validation logic
        let asset = Self(asset);
        asset.validate()?;
        Ok(asset)
    }

    // HELPER FUNCTIONS
    // --------------------------------------------------------------------------------------------

    /// Returns an error if:
    /// - The first bit of the asset is not ZERO.
    /// - The 32 least significant bits of the 3rd element are not set to 2^31.
    fn validate(&self) -> Result<(), AssetError> {
        let first_bit = self.0[0].as_int() >> 63;
        if first_bit != 0 {
            return Err(AssetError::non_fungible_asset_invalid_first_bit());
        }

        let tag = self.0[2].as_int() as u32;
        if tag != AccountId::NON_FUNGIBLE_FAUCET_TAG {
            return Err(AssetError::non_fungible_asset_invalid_tag(tag));
        }

        Ok(())
    }
}

impl Deref for NonFungibleAsset {
    type Target = Word;

    fn deref(&self) -> &Self::Target {
        &self.0
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
        result[8..16].copy_from_slice(&asset.0[1].as_int().to_le_bytes());
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
        if !faucet_id.is_non_fungible_faucet() {
            return Err(AssetError::not_a_non_fungible_faucet_id(faucet_id));
        }

        Ok(Self {
            faucet_id,
            asset_data,
        })
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
