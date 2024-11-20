use alloc::{boxed::Box, string::String};
use core::{error::Error, fmt};

use assembly::{diagnostics::reporting::PrintDiagnostic, Report};
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
    accounts::AccountType,
    notes::{NoteAssets, NoteExecutionHint, NoteTag, NoteType, Nullifier},
    ACCOUNT_UPDATE_MAX_SIZE, MAX_INPUTS_PER_NOTE, MAX_INPUT_NOTES_PER_TX, MAX_OUTPUT_NOTES_PER_TX,
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

#[derive(Debug, Error)]
pub enum AccountDeltaError {
    #[error("storage slot {0} was updated as a value and as a map")]
    StorageSlotUsedAsDifferentTypes(u8),
    #[error("non fungible vault can neither be added nor removed twice")]
    DuplicateNonFungibleVaultUpdate(NonFungibleAsset),
    #[error("fungible asset issued by faucet {faucet_id} has delta {delta} which overflows when added to current value {current}")]
    FungibleAssetDeltaOverflow {
        faucet_id: AccountId,
        current: i64,
        delta: i64,
    },
    #[error("account update of type `{left_update_type}` cannot be merged with account update of type `{right_update_type}`")]
    IncompatibleAccountUpdates {
        left_update_type: &'static str,
        right_update_type: &'static str,
    },
    #[error("account delta could not be applied to account {account_id}")]
    AccountDeltaApplicationFailed {
        account_id: AccountId,
        source: AccountError,
    },
    #[error("inconsistent nonce update: {0}")]
    InconsistentNonceUpdate(String),
    #[error("account id {0} in fungible asset delta is not of type fungible faucet")]
    NotAFungibleFaucetId(AccountId),
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
      id_type = .0.account_type(),
      expected_ty = AccountType::FungibleFaucet
    )]
    FungibleFaucetIdTypeMismatch(AccountId),
    #[error(
      "faucet id {0} of type {id_type:?} must be of type {expected_ty:?} for non fungible assets",
      id_type = .0.account_type(),
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
    #[error("duplicate fungible asset from issuer {0} in note")]
    DuplicateFungibleAsset(AccountId),
    #[error("duplicate non fungible asset {0} in note")]
    DuplicateNonFungibleAsset(NonFungibleAsset),
    #[error("note type {0:?} is inconsistent with note tag {1}")]
    InconsistentNoteTag(NoteType, u64),
    #[error("adding fungible asset amounts would exceed maximum allowed amount")]
    AddFungibleAssetBalanceError(#[source] AssetError),
    #[error("note sender is not a valid account id")]
    NoteSenderInvalidAccountId(#[source] AccountError),
    #[error("note tag use case {0} must be less than 2^{exp}", exp = NoteTag::MAX_USE_CASE_ID_EXPONENT)]
    NoteTagUseCaseTooLarge(u16),
    #[error(
        "note execution hint tag {0} must be in range {from}..={to}",
        from = NoteExecutionHint::NONE_TAG,
        to = NoteExecutionHint::ON_BLOCK_SLOT_TAG,
    )]
    NoteExecutionHintTagOutOfRange(u8),
    #[error("invalid note execution hint payload {1} for tag {0}")]
    InvalidNoteExecutionHintPayload(u8, u32),
    #[error("note type {0:b} does not match any of the valid note types {public}, {private} or {encrypted}",
      public = NoteType::Public as u8,
      private = NoteType::Private as u8,
      encrypted = NoteType::Encrypted as u8,
    )]
    InvalidNoteType(u64),
    #[error("note location index {node_index_in_block} is out of bounds 0..={highest_index}")]
    NoteLocationIndexOutOfBounds {
        node_index_in_block: u16,
        highest_index: usize,
    },
    #[error("note network execution requires account stored on chain")]
    NetworkExecutionRequiresOnChainAccount,
    #[error("note network execution requires a public note but note is of type {0:?}")]
    NetworkExecutionRequiresPublicNote(NoteType),
    #[error("failed to assemble note script:\n{}", PrintDiagnostic::new(.0))]
    NoteScriptAssemblyError(Report),
    #[error("failed to deserialize note script")]
    NoteScriptDeserializationError(#[source] DeserializationError),
    #[error("public use case requires a public note but note is of type {0:?}")]
    PublicUseCaseRequiresPublicNote(NoteType),
    #[error("note contains {0} assets which exceeds the maximum of {max}", max = NoteAssets::MAX_NUM_ASSETS)]
    TooManyAssets(usize),
    #[error("note contains {0} inputs which exceeds the maximum of {max}", max = MAX_INPUTS_PER_NOTE)]
    TooManyInputs(usize),
}

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

#[derive(Debug, Error)]
pub enum TransactionScriptError {
    #[error("failed to assemble transaction script:\n{}", PrintDiagnostic::new(.0))]
    AssemblyError(Report),
}

// TRANSACTION INPUT ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum TransactionInputError {
    #[error("account seed must be provided for new accounts")]
    AccountSeedNotProvidedForNewAccount,
    #[error("account seed must not be provided for existing accounts")]
    AccountSeedProvidedForExistingAccount,
    #[error("transaction input note with nullifier {0} is a duplicate")]
    DuplicateInputNote(Nullifier),
    #[error("ID {expected} of the new account does not match the ID {actual} computed from the provided seed")]
    InconsistentAccountSeed { expected: AccountId, actual: AccountId },
    #[error("chain mmr has length {actual} which does not match block number {expected} ")]
    InconsistentChainLength { expected: u32, actual: u32 },
    #[error("chain mmr has root {actual} which does not match block header's root {expected}")]
    InconsistentChainRoot { expected: Digest, actual: Digest },
    #[error("block in which input note with id {0} was created is not in chain mmr")]
    InputNoteBlockNotInChainMmr(NoteId),
    #[error("input note with id {0} was not created in block {1}")]
    InputNoteNotInBlock(NoteId, u32),
    #[error("account id computed from seed is invalid")]
    InvalidAccountIdSeed(#[source] AccountError),
    #[error(
        "total number of input notes is {0} which exceeds the maximum of {MAX_INPUT_NOTES_PER_TX}"
    )]
    TooManyInputNotes(usize),
}

// TRANSACTION OUTPUT ERROR
// ===============================================================================================

#[derive(Debug, Error)]
pub enum TransactionOutputError {
    #[error("transaction output note with id {0} is a duplicate")]
    DuplicateOutputNote(NoteId),
    #[error("final account hash is not in the advice map")]
    FinalAccountHashMissingInAdviceMap,
    #[error("failed to parse final account header")]
    FinalAccountHeaderParseFailure(#[source] AccountError),
    #[error("output notes commitment {expected} from kernel does not match computed commitment {actual}")]
    OutputNotesCommitmentInconsistent { expected: Digest, actual: Digest },
    #[error("transaction kernel output stack is invalid: {0}")]
    OutputStackInvalid(String),
    #[error("total number of output notes is {0} which exceeds the maximum of {MAX_OUTPUT_NOTES_PER_TX}")]
    TooManyOutputNotes(usize),
}

// PROVEN TRANSACTION ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum ProvenTransactionError {
    #[error("proven transaction's final account hash {tx_final_hash} and account details hash {details_hash} must match")]
    AccountFinalHashMismatch {
        tx_final_hash: Digest,
        details_hash: Digest,
    },
    #[error("proven transaction's final account id {tx_account_id} and account details id {details_account_id} must match")]
    AccountIdMismatch {
        tx_account_id: AccountId,
        details_account_id: AccountId,
    },
    #[error("failed to construct input notes for proven transaction")]
    InputNotesError(TransactionInputError),
    #[error("off-chain account {0} should not have account details")]
    OffChainAccountWithDetails(AccountId),
    #[error("on-chain account {0} is missing its account details")]
    OnChainAccountMissingDetails(AccountId),
    #[error("new on-chain account {0} is missing its account details")]
    NewOnChainAccountRequiresFullDetails(AccountId),
    #[error(
        "existing on-chain account {0} should only provide delta updates instead of full details"
    )]
    ExistingOnChainAccountRequiresDeltaDetails(AccountId),
    #[error("failed to construct output notes for proven transaction")]
    OutputNotesError(TransactionOutputError),
    #[error(
      "account update of size {update_size} for account {account_id} exceeds maximum update size of {ACCOUNT_UPDATE_MAX_SIZE}",
    )]
    AccountUpdateSizeLimitExceeded {
        account_id: AccountId,
        update_size: usize,
    },
}

// BLOCK VALIDATION ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum BlockError {
    #[error("duplicate note with id {0} in the block")]
    DuplicateNoteFound(NoteId),
    #[error("too many accounts updated in the block (max: {MAX_ACCOUNTS_PER_BLOCK}, actual: {0})")]
    TooManyAccountUpdates(usize),
    #[error("too many notes in the batch (max: {MAX_OUTPUT_NOTES_PER_BATCH}, actual: {0})")]
    TooManyNotesInBatch(usize),
    #[error("too many notes in the block (max: {MAX_OUTPUT_NOTES_PER_BLOCK}, actual: {0})")]
    TooManyNotesInBlock(usize),
    #[error("too many nullifiers in the block (max: {MAX_INPUT_NOTES_PER_BLOCK}, actual: {0})")]
    TooManyNullifiersInBlock(usize),
    #[error(
        "too many transaction batches in the block (max: {MAX_BATCHES_PER_BLOCK}, actual: {0})"
    )]
    TooManyTransactionBatches(usize),
}
