use super::{
    accounts::AccountId,
    assets::{Asset, FungibleAsset, NonFungibleAsset},
    crypto::merkle::MerkleError,
    utils::string::String,
    Digest, Word,
};
use assembly::AssemblyError;
use core::fmt;
use vm_processor::DeserializationError;

// ACCOUNT ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AccountError {
    AccountCodeAssemblerError(AssemblyError),
    AccountCodeNoProcedures,
    AccountCodeTooManyProcedures {
        max: usize,
        actual: usize,
    },
    AccountIdInvalidFieldElement(String),
    AccountIdTooFewOnes,
    AddFungibleAssetBalanceError(AssetError),
    ApplyStorageSlotsDiffFailed(MerkleError),
    ApplyStorageStoreDiffFailed(MerkleError),
    DuplicateAsset(MerkleError),
    DuplicateNonFungibleAsset(NonFungibleAsset),
    DuplicateStorageItems(MerkleError),
    FungibleAssetNotFound(FungibleAsset),
    FungibleFaucetIdInvalidFirstBit,
    FungibleFaucetInvalidMetadata(String),
    HexParseError(String),
    InconsistentAccountIdSeed {
        expected: AccountId,
        actual: AccountId,
    },
    NonceMustBeMonotonicallyIncreasing(u64, u64),
    NonFungibleAssetNotFound(NonFungibleAsset),
    NotAFungibleFaucetId(AccountId),
    NotANonFungibleAsset(Asset),
    SeedDigestTooFewTrailingZeros {
        expected: u32,
        actual: u32,
    },
    SetStoreNodeFailed(MerkleError),
    StubDataIncorrectLength(usize, usize),
    SubtractFungibleAssetBalanceError(AssetError),
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

    pub fn not_a_fungible_faucet_id(account_id: AccountId) -> Self {
        Self::NotAFungibleFaucetId(account_id)
    }

    pub fn not_a_non_fungible_asset(asset: Asset) -> Self {
        Self::NotANonFungibleAsset(asset)
    }
}

impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
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

// NOTE ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NoteError {
    NoteDeserializationError(DeserializationError),
    DuplicateFungibleAsset(AccountId),
    DuplicateNonFungibleAsset(NonFungibleAsset),
    EmptyAssetList,
    InconsistentStubHash(Digest, Digest),
    InconsistentStubNumAssets(u64, u64),
    InconsistentStubVaultHash(Digest, Digest),
    InvalidStubDataLen(usize),
    InvalidOriginIndex(String),
    InvalidVaultDataLen(usize),
    InvalidVaultAssetData(AssetError),
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

// TRANSACTION SCRIPT ERROR
// ================================================================================================

#[derive(Debug)]
pub enum TransactionScriptError {
    ScriptCompilationError(AssemblyError),
}

impl fmt::Display for TransactionScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScriptCompilationError(err) => {
                write!(f, "transaction script compilation error: {}", err)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionScriptError {}

// PREPARED TRANSACTION ERROR
// ===============================================================================================
#[derive(Debug)]
pub enum PreparedTransactionError {
    InvalidAccountIdSeedError(AccountError),
    AccountIdSeedNoteProvided,
}

impl fmt::Display for PreparedTransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PreparedTransactionError {}

// EXECUTED TRANSACTION ERROR
// ===============================================================================================
#[derive(Debug)]
pub enum ExecutedTransactionError {
    InvalidAccountIdSeedError(AccountError),
    AccountIdSeedNoteProvided,
}

impl fmt::Display for ExecutedTransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ExecutedTransactionError {}

// TRANSACTION RESULT ERROR
// ================================================================================================
#[derive(Debug)]
pub enum TransactionResultError {
    CreatedNoteDataNotFound,
    CreatedNoteDataInvalid(NoteError),
    CreatedNotesCommitmentInconsistent(Digest, Digest),
    FinalAccountDataNotFound,
    FinalAccountStubDataInvalid(AccountError),
    InconsistentAccountCodeHash(Digest, Digest),
    ExtractAccountStorageSlotsDeltaFailed(MerkleError),
    ExtractAccountStorageStoreDeltaFailed(MerkleError),
    ExtractAccountVaultLeavesDeltaFailed(MerkleError),
    UpdatedAccountCodeInvalid(AccountError),
}

impl fmt::Display for TransactionResultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionResultError {}

// TRANSACTION WITNESS ERROR
// ================================================================================================
#[derive(Debug)]
pub enum TransactionWitnessError {
    ConsumedNoteDataNotFound,
    InvalidConsumedNoteDataLength,
}

impl fmt::Display for TransactionWitnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionWitnessError {}
