use alloc::string::String;
use core::fmt;

use assembly::AssemblyError;
use vm_processor::DeserializationError;

use super::{
    accounts::{AccountId, StorageSlotType},
    assets::{Asset, FungibleAsset, NonFungibleAsset},
    crypto::merkle::MerkleError,
    notes::NoteId,
    Digest, Word,
};
use crate::utils::collections::*;

// ACCOUNT ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AccountError {
    AccountCodeAssemblerError(AssemblyError),
    AccountCodeNoProcedures,
    AccountCodeTooManyProcedures { max: usize, actual: usize },
    AccountIdInvalidFieldElement(String),
    AccountIdTooFewOnes,
    AssetVaultUpdateError(AssetVaultError),
    DuplicateStorageItems(MerkleError),
    FungibleFaucetIdInvalidFirstBit,
    FungibleFaucetInvalidMetadata(String),
    HexParseError(String),
    InconsistentAccountIdSeed { expected: AccountId, actual: AccountId },
    NonceNotMonotonicallyIncreasing { current: u64, new: u64 },
    SeedDigestTooFewTrailingZeros { expected: u32, actual: u32 },
    StorageSlotInvalidValueArity { slot: u8, expected: u8, actual: u8 },
    StorageSlotIsReserved(u8),
    StorageSlotNotValueSlot(u8, StorageSlotType),
    StubDataIncorrectLength(usize, usize),
}

impl AccountError {
    pub fn account_id_invalid_field_element(msg: String) -> Self {
        Self::AccountIdInvalidFieldElement(msg)
    }

    pub fn account_id_too_few_ones() -> Self {
        Self::AccountIdTooFewOnes
    }

    pub fn seed_digest_too_few_trailing_zeros(expected: u32, actual: u32) -> Self {
        Self::SeedDigestTooFewTrailingZeros { expected, actual }
    }

    pub fn fungible_faucet_id_invalid_first_bit() -> Self {
        Self::FungibleFaucetIdInvalidFirstBit
    }
}

impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AccountError {}

// ACCOUNT DELTA ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AccountDeltaError {
    DuplicateStorageItemUpdate(usize),
    DuplicateVaultUpdate(Asset),
    InconsistentNonceUpdate(String),
    ImmutableStorageSlot(usize),
    TooManyAddedAsset { actual: usize, max: usize },
    TooManyClearedStorageItems { actual: usize, max: usize },
    TooManyRemovedAssets { actual: usize, max: usize },
    TooManyUpdatedStorageItems { actual: usize, max: usize },
}

#[cfg(feature = "std")]
impl std::error::Error for AccountDeltaError {}

impl fmt::Display for AccountDeltaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// ASSET ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AssetError {
    AmountTooBig(u64),
    AssetAmountNotSufficient(u64, u64),
    FungibleAssetInvalidFirstBit,
    FungibleAssetInvalidTag(u32),
    FungibleAssetInvalidWord(Word),
    InconsistentFaucetIds(AccountId, AccountId),
    InvalidAccountId(String),
    InvalidFieldElement(String),
    NonFungibleAssetInvalidFirstBit,
    NonFungibleAssetInvalidTag(u32),
    NotAFungibleFaucetId(AccountId),
    NotANonFungibleFaucetId(AccountId),
    NotAnAsset(Word),
    TokenSymbolError(String),
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

    pub fn fungible_asset_invalid_word(word: Word) -> Self {
        Self::FungibleAssetInvalidWord(word)
    }

    pub fn inconsistent_faucet_ids(id1: AccountId, id2: AccountId) -> Self {
        Self::InconsistentFaucetIds(id1, id2)
    }

    pub fn invalid_account_id(err: String) -> Self {
        Self::InvalidAccountId(err)
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AssetError {}

// ASSET VAULT ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetVaultError {
    AddFungibleAssetBalanceError(AssetError),
    DuplicateAsset(MerkleError),
    DuplicateNonFungibleAsset(NonFungibleAsset),
    FungibleAssetNotFound(FungibleAsset),
    NotANonFungibleAsset(Asset),
    NotAFungibleFaucetId(AccountId),
    NonFungibleAssetNotFound(NonFungibleAsset),
    SubtractFungibleAssetBalanceError(AssetError),
}

impl fmt::Display for AssetVaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AssetVaultError {}

// NOTE ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NoteError {
    DuplicateFungibleAsset(AccountId),
    DuplicateNonFungibleAsset(NonFungibleAsset),
    EmptyAssetList,
    InconsistentStubId(NoteId, NoteId),
    InconsistentStubAssetHash(Digest, Digest),
    InvalidStubDataLen(usize),
    InvalidOriginIndex(String),
    InvalidAssetData(AssetError),
    NoteDeserializationError(DeserializationError),
    NoteMetadataSenderInvalid(AccountError),
    ScriptCompilationError(AssemblyError),
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

    pub fn invalid_origin_index(msg: String) -> Self {
        Self::InvalidOriginIndex(msg)
    }

    pub fn too_many_assets(num_assets: usize) -> Self {
        Self::TooManyAssets(num_assets)
    }

    pub fn too_many_inputs(num_inputs: usize) -> Self {
        Self::TooManyInputs(num_inputs)
    }
}

impl fmt::Display for NoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NoteError {}

// CHAIN MMR ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ChainMmrError {
    BlockNumTooBig { chain_length: usize, block_num: u32 },
    DuplicateBlock { block_num: u32 },
    UntrackedBlock { block_num: u32 },
}

impl ChainMmrError {
    pub fn block_num_too_big(chain_length: usize, block_num: u32) -> Self {
        Self::BlockNumTooBig { chain_length, block_num }
    }

    pub fn duplicate_block(block_num: u32) -> Self {
        Self::DuplicateBlock { block_num }
    }

    pub fn untracked_block(block_num: u32) -> Self {
        Self::UntrackedBlock { block_num }
    }
}

impl fmt::Display for ChainMmrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ChainMmrError {}

// TRANSACTION SCRIPT ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionScriptError {
    ScriptCompilationError(AssemblyError),
}

impl fmt::Display for TransactionScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionScriptError {}

// TRANSACTION INPUT ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionInputError {
    AccountSeedNotProvidedForNewAccount,
    AccountSeedProvidedForExistingAccount,
    DuplicateInputNote(Digest),
    InconsistentChainLength { expected: u32, actual: u32 },
    InconsistentChainRoot { expected: Digest, actual: Digest },
    InputNoteBlockNotInChainMmr(NoteId),
    InputNoteNotInBlock(NoteId, u32),
    InvalidAccountSeed(AccountError),
    TooManyInputNotes { max: usize, actual: usize },
}

impl fmt::Display for TransactionInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionInputError {}

// TRANSACTION OUTPUT ERROR
// ===============================================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionOutputError {
    DuplicateOutputNote(NoteId),
    FinalAccountDataNotFound,
    FinalAccountStubDataInvalid(AccountError),
    OutputNoteDataNotFound,
    OutputNoteDataInvalid(NoteError),
    OutputNotesCommitmentInconsistent(Digest, Digest),
    TooManyOutputNotes { max: usize, actual: usize },
}

impl fmt::Display for TransactionOutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionOutputError {}

// PROVEN TRANSACTION ERROR
// ================================================================================================

#[derive(Debug)]
pub enum ProvenTransactionError {
    AccountFinalHashMismatch(Digest, Digest),
    AccountIdMismatch(AccountId, AccountId),
    InputNotesError(TransactionInputError),
    NoteDetailsForUnknownNotes(Vec<NoteId>),
    OffChainAccountWithDetails(AccountId),
    OnChainAccountMissingDetails(AccountId),
    NewOnChainAccountRequiresFullDetails(AccountId),
    ExistingOnChainAccountRequiresDeltaDetails(AccountId),
    OutputNotesError(TransactionOutputError),
}

impl fmt::Display for ProvenTransactionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProvenTransactionError::AccountFinalHashMismatch(tx_digest, details_hash) => {
                write!(f, "Proven transaction account_final_hash {} and account_details.hash must match {}.", tx_digest, details_hash)
            },
            ProvenTransactionError::AccountIdMismatch(tx_id, details_id) => {
                write!(
                    f,
                    "Provent transaction account_id {} and account_details.id must match {}.",
                    tx_id, details_id,
                )
            },
            ProvenTransactionError::InputNotesError(inner) => {
                write!(f, "Invalid input notes: {}", inner)
            },
            ProvenTransactionError::NoteDetailsForUnknownNotes(note_ids) => {
                write!(f, "Note details for unknown note ids: {:?}", note_ids)
            },
            ProvenTransactionError::OffChainAccountWithDetails(account_id) => {
                write!(f, "Off-chain account {} should not have account details", account_id)
            },
            ProvenTransactionError::OnChainAccountMissingDetails(account_id) => {
                write!(f, "On-chain account {} missing account details", account_id)
            },
            ProvenTransactionError::OutputNotesError(inner) => {
                write!(f, "Invalid output notes: {}", inner)
            },
            ProvenTransactionError::NewOnChainAccountRequiresFullDetails(account_id) => {
                write!(f, "New on-chain account {} missing full details", account_id)
            },
            ProvenTransactionError::ExistingOnChainAccountRequiresDeltaDetails(account_id) => {
                write!(f, "Existing on-chain account {} should only provide deltas", account_id)
            },
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ProvenTransactionError {}
