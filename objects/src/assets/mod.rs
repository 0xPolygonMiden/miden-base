use super::{
    AccountId, AccountType, AssetError, Felt, Hasher, StarkField, ToString, Vec, Word, ZERO,
};
use core::{fmt, ops::Deref};

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
pub enum Asset {
    Fungible(FungibleAsset),
    NonFungible(NonFungibleAsset),
}

impl Asset {
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
        key[3] = *self.faucet_id;
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
        result[3] = *asset.faucet_id;
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
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

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
        data_hash[1] = *faucet_id;

        // set the first bit of the asset to 0; we can do this because setting the first bit to 0
        // will always result in a valid field element.
        data_hash[3] = Felt::new((data_hash[3].as_int() << 1) >> 1);

        // construct an asset
        let asset = Self(data_hash);

        Ok(asset)
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------
    pub fn vault_key(&self) -> Word {
        self.0
    }

    // HELPER FUNCTIONS
    // --------------------------------------------------------------------------------------------

    /// Validates this non-fungible asset.
    /// # Errors
    /// Returns an error if:
    /// - The faucet_id is not a valid non-fungible faucet ID.
    /// - The most significant bit of the asset is not ZERO.
    fn validate(&self) -> Result<(), AssetError> {
        let faucet_id = AccountId::try_from(self.0[1])
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
