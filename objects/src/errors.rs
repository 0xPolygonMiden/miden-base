use super::{assets::Asset, assets::NonFungibleAsset, AccountId, Word};
use assembly::ParsingError;
use core::fmt;

// ACCOUNT ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AccountError {
    AccountIdInvalidFieldElement(String),
    AccountIdTooFewTrailingZeros,
    CodeParsingFailed(ParsingError),
    FungibleFaucetIdInvalidFirstBit,
    NotAFungibleFaucetId(AccountId),
    NotANonFungibleAsset(Asset),
}

impl AccountError {
    pub fn account_id_invalid_field_element(msg: String) -> Self {
        Self::AccountIdInvalidFieldElement(msg)
    }

    pub fn account_id_too_few_trailing_zeros() -> Self {
        Self::AccountIdTooFewTrailingZeros
    }

    pub fn fungible_faucet_id_invalid_first_bit() -> Self {
        Self::FungibleFaucetIdInvalidFirstBit
    }

    pub fn not_a_fungible_faucet_id(account_id: AccountId) -> Self {
        Self::NotAFungibleFaucetId(account_id)
    }

    pub fn not_a_non_fungible_asset(asset: Asset) -> Self {
        Self::NotANonFungibleAsset(asset)
    }
}

impl From<ParsingError> for AccountError {
    fn from(err: ParsingError) -> Self {
        Self::CodeParsingFailed(err)
    }
}

impl fmt::Display for AccountError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AccountError {}

// ASSET ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AssetError {
    AmountTooBig(u64),
    AssetAmountNotSufficient(u64, u64),
    FungibleAssetInvalidFirstBit,
    FungibleAssetInvalidTag(u32),
    InconsistentFaucetIds(AccountId, AccountId),
    InvalidAccountId(String),
    InvalidFieldElement(String),
    NonFungibleAssetInvalidFirstBit,
    NonFungibleAssetInvalidTag(u32),
    NotAFungibleFaucetId(AccountId),
    NotANonFungibleFaucetId(AccountId),
    NotAnAsset(Word),
}

impl AssetError {
    pub fn amount_too_big(value: u64) -> Self {
        Self::AmountTooBig(value)
    }

    pub fn asset_amount_not_sufficient(available: u64, requested: u64) -> Self {
        Self::AssetAmountNotSufficient(available, requested)
    }

    pub fn fungible_asset_invalid_first_bit() -> Self {
        Self::FungibleAssetInvalidFirstBit
    }

    pub fn fungible_asset_invalid_tag(tag: u32) -> Self {
        Self::FungibleAssetInvalidTag(tag)
    }

    pub fn inconsistent_faucet_ids(id1: AccountId, id2: AccountId) -> Self {
        Self::InconsistentFaucetIds(id1, id2)
    }

    pub fn invalid_account_id(err: AccountError) -> Self {
        Self::InvalidAccountId(err.to_string())
    }

    pub fn invalid_field_element(msg: String) -> Self {
        Self::InvalidFieldElement(msg)
    }

    pub fn non_fungible_asset_invalid_first_bit() -> Self {
        Self::NonFungibleAssetInvalidFirstBit
    }

    pub fn non_fungible_asset_invalid_tag(tag: u32) -> Self {
        Self::NonFungibleAssetInvalidTag(tag)
    }

    pub fn not_a_fungible_faucet_id(id: AccountId) -> Self {
        Self::NotAFungibleFaucetId(id)
    }

    pub fn not_a_non_fungible_faucet_id(id: AccountId) -> Self {
        Self::NotANonFungibleFaucetId(id)
    }

    pub fn not_an_asset(value: Word) -> Self {
        Self::NotAnAsset(value)
    }
}

impl fmt::Display for AssetError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AssetError {}

// NOTE ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NoteError {
    DuplicateFungibleAsset(AccountId),
    DuplicateNonFungibleAsset(NonFungibleAsset),
    EmptyAssetList,
    TooManyAssets(usize),
    TooManyInputs(usize),
}

impl NoteError {
    pub fn duplicate_fungible_asset(faucet_id: AccountId) -> Self {
        Self::DuplicateFungibleAsset(faucet_id)
    }

    pub fn duplicate_non_fungible_asset(asset: NonFungibleAsset) -> Self {
        Self::DuplicateNonFungibleAsset(asset)
    }

    pub fn empty_asset_list() -> Self {
        Self::EmptyAssetList
    }

    pub fn too_many_assets(num_assets: usize) -> Self {
        Self::TooManyAssets(num_assets)
    }

    pub fn too_many_inputs(num_inputs: usize) -> Self {
        Self::TooManyInputs(num_inputs)
    }
}

impl fmt::Display for NoteError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NoteError {}
