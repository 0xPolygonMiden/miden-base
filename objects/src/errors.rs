use alloc::{boxed::Box, string::String, vec::Vec};
use core::{error::Error, fmt};

use thiserror::Error;
use vm_processor::DeserializationError;

use super::{
    accounts::{AccountId, StorageSlotType},
    assets::{FungibleAsset, NonFungibleAsset},
    crypto::merkle::MerkleError,
    notes::NoteId,
    Digest, Word, MAX_ACCOUNTS_PER_BLOCK, MAX_BATCHES_PER_BLOCK, MAX_INPUT_NOTES_PER_BLOCK,
    MAX_OUTPUT_NOTES_PER_BATCH, MAX_OUTPUT_NOTES_PER_BLOCK,
};
use crate::{
    accounts::{delta::AccountUpdateDetails, AccountType},
    notes::NoteType,
    ACCOUNT_UPDATE_MAX_SIZE,
};

// ACCOUNT ERROR
// ================================================================================================

#[derive(Debug)]
pub enum AccountError {
    AccountCodeAssemblyError(String), // TODO: use Report
    AccountCodeMergeError(String),    // TODO: use MastForestError once it implements Clone
    AccountCodeDeserializationError(DeserializationError),
    AccountCodeNoProcedures,
    AccountCodeTooManyProcedures {
        max: usize,
        actual: usize,
    },
    AccountCodeProcedureInvalidStorageOffset,
    AccountCodeProcedureInvalidStorageSize,
    AccountCodeProcedureInvalidPadding,
    AccountIdInvalidFieldElement(String),
    AccountIdTooFewOnes(u32, u32),
    AssetVaultUpdateError(AssetVaultError),
    BuildError(String, Option<Box<AccountError>>),
    DuplicateStorageItems(MerkleError),
    FungibleFaucetIdInvalidFirstBit,
    FungibleFaucetInvalidMetadata(String),
    HeaderDataIncorrectLength(usize, usize),
    HexParseError(String),
    InvalidAccountStorageMode,
    MapsUpdateToNonMapsSlot(u8, StorageSlotType),
    NonceNotMonotonicallyIncreasing {
        current: u64,
        new: u64,
    },
    SeedDigestTooFewTrailingZeros {
        expected: u32,
        actual: u32,
    },
    StorageSlotNotMap(u8),
    StorageSlotNotValue(u8),
    StorageIndexOutOfBounds {
        max: u8,
        actual: u8,
    },
    StorageTooManySlots(u64),
    StorageOffsetOutOfBounds {
        max: u8,
        actual: u16,
    },
    PureProcedureWithStorageOffset,
    UnsupportedComponentForAccountType {
        account_type: AccountType,
        component_index: usize,
    },
}

impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountError::BuildError(msg, err) => {
                write!(f, "account build error: {msg}")?;
                if let Some(err) = err {
                    write!(f, ": {err}")?;
                }
                Ok(())
            },
            other => write!(f, "{other:?}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AccountError {}

// ACCOUNT DELTA ERROR
// ================================================================================================

#[derive(Debug)]
pub enum AccountDeltaError {
    DuplicateStorageItemUpdate(usize),
    DuplicateNonFungibleVaultUpdate(NonFungibleAsset),
    FungibleAssetDeltaOverflow {
        faucet_id: AccountId,
        this: i64,
        other: i64,
    },
    IncompatibleAccountUpdates(AccountUpdateDetails, AccountUpdateDetails),
    InconsistentNonceUpdate(String),
    NotAFungibleFaucetId(AccountId),
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

#[derive(Debug, Error)]
pub enum AssetError {
    #[error(
      "fungible asset amount {0} exceeds the max allowed amount of {max_amount}",
      max_amount = FungibleAsset::MAX_AMOUNT
    )]
    FungibleAssetAmountTooBig(u64),
    #[error("subtracting {subtrahend} from fungible asset amount {minuend} would overflow")]
    FungibleAssetAmountNotSufficient { minuend: u64, subtrahend: u64 },
    #[error("fungible asset word {0:?} does not contain expected ZEROs at word index 1 and 2")]
    FungibleAssetExpectedZeroes(Word),
    #[error("cannot add fungible asset with issuer {other_issuer} to fungible asset with issuer {original_issuer}")]
    FungibleAssetInconsistentFaucetIds {
        original_issuer: AccountId,
        other_issuer: AccountId,
    },
    #[error("faucet account id in asset is invalid")]
    InvalidFaucetAccountId(#[source] Box<dyn Error>),
    #[error(
      "faucet id {0} of type {id_type:?} must be of type {expected_ty:?} for fungible assets",
      id_type = _0.account_type(),
      expected_ty = AccountType::FungibleFaucet
    )]
    FungibleFaucetIdTypeMismatch(AccountId),
    #[error(
      "faucet id {0} of type {id_type:?} must be of type {expected_ty:?} for non fungible assets",
      id_type = _0.account_type(),
      expected_ty = AccountType::NonFungibleFaucet
    )]
    NonFungibleFaucetIdTypeMismatch(AccountId),
    #[error("{0}")]
    TokenSymbolError(String),
}

// ASSET VAULT ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum AssetVaultError {
    #[error("adding fungible asset amounts would exceed maximum allowed amount")]
    AddFungibleAssetBalanceError(#[source] AssetError),
    #[error("provided assets contain duplicates")]
    DuplicateAsset(#[source] MerkleError),
    #[error("non fungible asset {0} already exists in the vault")]
    DuplicateNonFungibleAsset(NonFungibleAsset),
    #[error("fungible asset {0} does not exist in the vault")]
    FungibleAssetNotFound(FungibleAsset),
    #[error("faucet id {0} is not a fungible faucet id")]
    NotAFungibleFaucetId(AccountId),
    #[error("non fungible asset {0} does not exist in the vault")]
    NonFungibleAssetNotFound(NonFungibleAsset),
    #[error("subtracting fungible asset amounts would underflow")]
    SubtractFungibleAssetBalanceError(#[source] AssetError),
}

// NOTE ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum NoteError {
    DuplicateFungibleAsset(AccountId),
    DuplicateNonFungibleAsset(NonFungibleAsset),
    InconsistentNoteTag(NoteType, u64),
    InvalidAssetData(AssetError),
    InvalidNoteSender(AccountError),
    InvalidNoteTagUseCase(u16),
    InvalidNoteExecutionHintTag(u8),
    InvalidNoteExecutionHintPayload(u8, u32),
    InvalidNoteType(NoteType),
    InvalidNoteTypeValue(u64),
    InvalidLocationIndex(String),
    InvalidStubDataLen(usize),
    NetworkExecutionRequiresOnChainAccount,
    NetworkExecutionRequiresPublicNote(NoteType),
    NoteDeserializationError(DeserializationError),
    NoteScriptAssemblyError(String), // TODO: use Report
    NoteScriptDeserializationError(DeserializationError),
    PublicUseCaseRequiresPublicNote(NoteType),
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

    pub fn invalid_location_index(msg: String) -> Self {
        Self::InvalidLocationIndex(msg)
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

#[derive(Debug, Error)]
pub enum ChainMmrError {
    #[error("block num {block_num} exceeds chain length {chain_length} implied by the chain MMR")]
    BlockNumTooBig { chain_length: usize, block_num: u32 },
    #[error("duplicate block {block_num} in chain MMR")]
    DuplicateBlock { block_num: u32 },
    #[error("chain MMR does not track authentication paths for block {block_num}")]
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

// TRANSACTION SCRIPT ERROR
// ================================================================================================

#[derive(Debug)]
pub enum TransactionScriptError {
    AssemblyError(String), // TODO: change to Report
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

#[derive(Debug)]
pub enum TransactionInputError {
    AccountSeedNotProvidedForNewAccount,
    AccountSeedProvidedForExistingAccount,
    DuplicateInputNote(Digest),
    InconsistentAccountSeed { expected: AccountId, actual: AccountId },
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

#[derive(Debug)]
pub enum TransactionOutputError {
    DuplicateOutputNote(NoteId),
    FinalAccountDataNotFound,
    FinalAccountHeaderDataInvalid(AccountError),
    OutputNoteDataNotFound,
    OutputNoteDataInvalid(NoteError),
    OutputNotesCommitmentInconsistent(Digest, Digest),
    OutputStackInvalid(String),
    TooManyOutputNotes(usize),
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
    AccountUpdateSizeLimitExceeded(AccountId, usize),
}

impl fmt::Display for ProvenTransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProvenTransactionError::AccountFinalHashMismatch(account_final_hash, details_hash) => {
                write!(f, "Proven transaction account_final_hash {account_final_hash} and account_details.hash must match {details_hash}.")
            },
            ProvenTransactionError::AccountIdMismatch(tx_id, details_id) => {
                write!(
                    f,
                    "Proven transaction account_id {tx_id} and account_details.id must match {details_id}.",
                )
            },
            ProvenTransactionError::InputNotesError(inner) => {
                write!(f, "Invalid input notes: {inner}")
            },
            ProvenTransactionError::NoteDetailsForUnknownNotes(note_ids) => {
                write!(f, "Note details for unknown note ids: {note_ids:?}")
            },
            ProvenTransactionError::OffChainAccountWithDetails(account_id) => {
                write!(f, "Off-chain account {account_id} should not have account details")
            },
            ProvenTransactionError::OnChainAccountMissingDetails(account_id) => {
                write!(f, "On-chain account {account_id} missing account details")
            },
            ProvenTransactionError::OutputNotesError(inner) => {
                write!(f, "Invalid output notes: {inner}")
            },
            ProvenTransactionError::NewOnChainAccountRequiresFullDetails(account_id) => {
                write!(f, "New on-chain account {account_id} missing full details")
            },
            ProvenTransactionError::ExistingOnChainAccountRequiresDeltaDetails(account_id) => {
                write!(f, "Existing on-chain account {account_id} should only provide deltas")
            },
            ProvenTransactionError::AccountUpdateSizeLimitExceeded(account_id, size) => {
                write!(f, "Update on account {account_id} of size {size} exceeds the allowed limit of {ACCOUNT_UPDATE_MAX_SIZE}")
            },
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ProvenTransactionError {}

// BLOCK VALIDATION ERROR
// ================================================================================================

#[derive(Debug)]
pub enum BlockError {
    DuplicateNoteFound(NoteId),
    TooManyAccountUpdates(usize),
    TooManyNotesInBatch(usize),
    TooManyNotesInBlock(usize),
    TooManyNullifiersInBlock(usize),
    TooManyTransactionBatches(usize),
}

impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockError::DuplicateNoteFound(id) => {
                write!(f, "Duplicate note {id} found in the block")
            },
            BlockError::TooManyAccountUpdates(actual) => {
                write!(f, "Too many accounts updated in a block. Max: {MAX_ACCOUNTS_PER_BLOCK}, actual: {actual}")
            },
            BlockError::TooManyNotesInBatch(actual) => {
                write!(f, "Too many notes in a batch. Max: {MAX_OUTPUT_NOTES_PER_BATCH}, actual: {actual}")
            },
            BlockError::TooManyNotesInBlock(actual) => {
                write!(f, "Too many notes in a block. Max: {MAX_OUTPUT_NOTES_PER_BLOCK}, actual: {actual}")
            },
            BlockError::TooManyNullifiersInBlock(actual) => {
                write!(
                    f,
                    "Too many nullifiers in a block. Max: {MAX_INPUT_NOTES_PER_BLOCK}, actual: {actual}"
                )
            },
            BlockError::TooManyTransactionBatches(actual) => {
                write!(
                    f,
                    "Too many transaction batches. Max: {MAX_BATCHES_PER_BLOCK}, actual: {actual}"
                )
            },
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BlockError {}
