use alloc::{string::String, vec::Vec};
use core::fmt;

use assembly::AssemblyError;
use vm_processor::DeserializationError;

use super::{
    accounts::{AccountId, StorageSlotType},
    assets::{Asset, FungibleAsset, NonFungibleAsset},
    crypto::{hash::rpo::RpoDigest, merkle::MerkleError},
    notes::NoteId,
    Digest, Word, MAX_BATCHES_PER_BLOCK, MAX_NOTES_PER_BATCH,
};
use crate::{
    accounts::{delta::AccountUpdateDetails, AccountType},
    notes::NoteType,
};

// ACCOUNT ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountError {
    AccountCodeAssemblerError(AssemblyError),
    AccountCodeNoProcedures,
    AccountCodeTooManyProcedures { max: usize, actual: usize },
    AccountCodeProcedureInvalidStorageOffset,
    AccountCodeProcedureInvalidPadding,
    AccountIdInvalidFieldElement(String),
    AccountIdTooFewOnes(u32, u32),
    AssetVaultUpdateError(AssetVaultError),
    DuplicateStorageItems(MerkleError),
    FungibleFaucetIdInvalidFirstBit,
    FungibleFaucetInvalidMetadata(String),
    HexParseError(String),
    InvalidAccountStorageType,
    MapsUpdateToNonMapsSlot(u8, StorageSlotType),
    NonceNotMonotonicallyIncreasing { current: u64, new: u64 },
    SeedDigestTooFewTrailingZeros { expected: u32, actual: u32 },
    StorageSlotInvalidValueArity { slot: u8, expected: u8, actual: u8 },
    StorageSlotIsReserved(u8),
    StorageSlotMapOrArrayNotAllowed(u8, StorageSlotType),
    StorageMapNotFound(u8),
    StorageMapTooManyMaps { expected: usize, actual: usize },
    StubDataIncorrectLength(usize, usize),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountDeltaError {
    UpdateIndexOutOfBounds(usize),
    DuplicateStorageItemUpdate(usize),
    DuplicateVaultUpdate(Asset),
    InconsistentNonceUpdate(String),
    ImmutableStorageSlot(usize),
    TooManyAddedAsset { actual: usize, max: usize },
    TooManyClearedStorageItems { actual: usize, max: usize },
    TooManyRemovedAssets { actual: usize, max: usize },
    TooManyUpdatedStorageItems { actual: usize, max: usize },
    DuplicateStorageMapLeaf { key: RpoDigest },
    AssetAmountTooBig(u64),
    IncompatibleAccountUpdates(AccountUpdateDetails, AccountUpdateDetails),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetError {
    AmountTooBig(u64),
    AssetAmountNotSufficient(u64, u64),
    FungibleAssetInvalidTag(u32),
    FungibleAssetInvalidWord(Word),
    InconsistentFaucetIds(AccountId, AccountId),
    InvalidAccountId(String),
    InvalidFieldElement(String),
    NonFungibleAssetInvalidTag(u32),
    NotAFungibleFaucetId(AccountId, AccountType),
    NotANonFungibleFaucetId(AccountId),
    NotAnAsset(Word),
    TokenSymbolError(String),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteError {
    DuplicateFungibleAsset(AccountId),
    DuplicateNonFungibleAsset(NonFungibleAsset),
    InconsistentNoteTag(NoteType, u64),
    InvalidAssetData(AssetError),
    InvalidNoteSender(AccountError),
    InvalidNoteTagUseCase(u16),
    InvalidNoteType(NoteType),
    InvalidNoteTypeValue(u64),
    InvalidOriginIndex(String),
    InvalidStubDataLen(usize),
    NetworkExecutionRequiresOnChainAccount,
    NetworkExecutionRequiresPublicNote(NoteType),
    NoteDeserializationError(DeserializationError),
    PublicUseCaseRequiresPublicNote(NoteType),
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionOutputError {
    DuplicateOutputNote(NoteId),
    FinalAccountDataNotFound,
    FinalAccountStubDataInvalid(AccountError),
    OutputNoteDataNotFound,
    OutputNoteDataInvalid(NoteError),
    OutputNotesCommitmentInconsistent(Digest, Digest),
    OutputStackInvalid(String),
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ProvenTransactionError {}

// BLOCK VALIDATION ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockError {
    DuplicateNoteFound(NoteId),
    TooManyNotesInBatch(usize),
    TooManyTransactionBatches(usize),
}

impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockError::DuplicateNoteFound(id) => {
                write!(f, "Duplicate note {id} found in the block")
            },
            BlockError::TooManyNotesInBatch(actual) => {
                write!(f, "Too many notes in a batch. Max: {MAX_NOTES_PER_BATCH}, actual: {actual}")
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
