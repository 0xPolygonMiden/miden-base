use alloc::string::String;
use core::fmt::{self, Display};

use miden_objects::{
    assembly::AssemblyError, notes::NoteId, Felt, NoteError, ProvenTransactionError,
    TransactionInputError, TransactionOutputError,
};
use miden_verifier::VerificationError;

use super::{AccountError, AccountId, Digest, ExecutionError};

// TRANSACTION COMPILER ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionCompilerError {
    AccountInterfaceNotFound(AccountId),
    BuildCodeBlockTableFailed(AssemblyError),
    CompileNoteScriptFailed(AssemblyError),
    CompileTxScriptFailed(AssemblyError),
    LoadAccountFailed(AccountError),
    NoteIncompatibleWithAccountInterface(Digest),
    NoteScriptError(NoteError),
    NoTransactionDriver,
    TxScriptIncompatibleWithAccountInterface(Digest),
}

impl fmt::Display for TransactionCompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionCompilerError {}

// TRANSACTION EXECUTOR ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionExecutorError {
    CompileNoteScriptFailed(TransactionCompilerError),
    CompileTransactionScriptFailed(TransactionCompilerError),
    CompileTransactionFailed(TransactionCompilerError),
    ExecuteTransactionProgramFailed(ExecutionError),
    FetchAccountCodeFailed(DataStoreError),
    FetchTransactionInputsFailed(DataStoreError),
    InconsistentAccountId {
        input_id: AccountId,
        output_id: AccountId,
    },
    InconsistentAccountNonceDelta {
        expected: Option<Felt>,
        actual: Option<Felt>,
    },
    InvalidTransactionOutput(TransactionOutputError),
    LoadAccountFailed(TransactionCompilerError),
}

impl fmt::Display for TransactionExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionExecutorError {}

// TRANSACTION PROVER ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionProverError {
    ProveTransactionProgramFailed(ExecutionError),
    InvalidAccountDelta(AccountError),
    InvalidTransactionOutput(TransactionOutputError),
    ProvenTransactionError(ProvenTransactionError),
}

impl Display for TransactionProverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionProverError::ProveTransactionProgramFailed(inner) => {
                write!(f, "Proving transaction failed: {}", inner)
            },
            TransactionProverError::InvalidAccountDelta(account_error) => {
                write!(f, "Applying account delta failed: {}", account_error)
            },
            TransactionProverError::InvalidTransactionOutput(inner) => {
                write!(f, "Transaction ouptut invalid: {}", inner)
            },
            TransactionProverError::ProvenTransactionError(inner) => {
                write!(f, "Building proven transaction error: {}", inner)
            },
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionProverError {}

// TRANSACTION VERIFIER ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionVerifierError {
    TransactionVerificationFailed(VerificationError),
    InsufficientProofSecurityLevel(u32, u32),
}

impl fmt::Display for TransactionVerifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionVerifierError {}

// DATA STORE ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataStoreError {
    AccountNotFound(AccountId),
    BlockNotFound(u32),
    InvalidTransactionInput(TransactionInputError),
    InternalError(String),
    NoteAlreadyConsumed(NoteId),
    NoteNotFound(NoteId),
}

impl fmt::Display for DataStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DataStoreError {}

// KERNEL ASSERTION ERROR
// ================================================================================================

const ERR_FAUCET_RESERVED_DATA_SLOT: u32 = 131072;
const ERR_ACCT_MUST_BE_A_FAUCET: u32 = 131073;
const ERR_P2ID_WRONG_NUMBER_OF_INPUTS: u32 = 131074;
const ERR_P2ID_TARGET_ACCT_MISMATCH: u32 = 131075;
const ERR_P2IDR_WRONG_NUMBER_OF_INPUTS: u32 = 131076;
const ERR_P2IDR_RECLAIM_ACCT_IS_NOT_SENDER: u32 = 131077;
const ERR_P2IDR_RECLAIM_HEIGHT_NOT_REACHED: u32 = 131078;
const ERR_SWAP_WRONG_NUMBER_OF_INPUTS: u32 = 131079;
const ERR_SWAP_WRONG_NUMBER_OF_ASSETS: u32 = 131080;
const ERR_KERNEL_TX_NONCE_DID_NOT_INCREASE: u32 = 131081;
const ERR_KERNEL_ASSET_MISMATCH: u32 = 131082;
const ERR_PROLOGUE_GLOBAL_INPUTS_MISMATCH: u32 = 131083;
const ERR_PROLOGUE_ACCT_STORAGE_MISMATCH: u32 = 131084;
const ERR_PROLOGUE_ACCT_STORAGE_ARITY_TOO_HIGH: u32 = 131085;
const ERR_PROLOGUE_ACCT_STORAGE_TYPE_INVALID: u32 = 131086;
const ERR_PROLOGUE_NEW_ACCT_VAULT_NOT_EMPTY: u32 = 131087;
const ERR_PROLOGUE_NEW_ACCT_INVALID_SLOT_TYPE: u32 = 131088;
const ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_NON_EMPTY_RESERVED_SLOT: u32 = 131089;
const ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_NON_ZERO_RESERVED_SLOT: u32 = 131090;
const ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_INVALID_TYPE_RESERVED_SLOT: u32 = 131091;
const ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_INVALID_RESERVED_SLOT: u32 = 131092;
const ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_NON_ZERO_RESERVED_SLOT: u32 = 131093;
const ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_INVALID_TYPE_RESERVED_SLOT: u32 = 131094;
const ERR_PROLOGUE_ACCT_HASH_MISMATCH: u32 = 131095;
const ERR_PROLOGUE_OLD_ACCT_NONCE_ZERO: u32 = 131096;
const ERR_PROLOGUE_ACCT_ID_MISMATCH: u32 = 131097;
const ERR_PROLOGUE_NOTE_MMR_DIGEST_MISMATCH: u32 = 131098;
const ERR_PROLOGUE_NOTE_TOO_MANY_INPUTS: u32 = 131099;
const ERR_PROLOGUE_NOTE_TOO_MANY_ASSETS: u32 = 131100;
const ERR_PROLOGUE_NOTE_CONSUMED_ASSETS_MISMATCH: u32 = 131101;
const ERR_PROLOGUE_TOO_MANY_INPUT_NOTES: u32 = 131102;
const ERR_PROLOGUE_INPUT_NOTES_NULLIFIER_COMMITMENT_MISMATCH: u32 = 131103;
const ERR_TX_OUTPUT_NOTES_OVERFLOW: u32 = 131104;
const ERR_BASIC_FUNGIBLE_MAX_SUPPLY_OVERFLOW: u32 = 131105;
const ERR_FAUCET_ISSUANCE_OVERFLOW: u32 = 131106;
const ERR_FAUCET_BURN_OVER_ISSUANCE: u32 = 131107;
const ERR_FAUCET_NON_FUNGIBLE_ALREADY_EXISTS: u32 = 131108;
const ERR_FAUCET_NON_FUNGIBLE_BURN_WRONG_TYPE: u32 = 131109;
const ERR_FAUCET_NONEXISTING_TOKEN: u32 = 131110;
const ERR_NOTE_INVALID_SENDER: u32 = 131111;
const ERR_NOTE_INVALID_VAULT: u32 = 131112;
const ERR_NOTE_INVALID_INPUTS: u32 = 131113;
const ERR_NOTE_TOO_MANY_ASSETS: u32 = 131114;
const ERR_VAULT_GET_BALANCE_WRONG_ASSET_TYPE: u32 = 131115;
const ERR_VAULT_HAS_NON_FUNGIBLE_WRONG_ACCOUNT_TYPE: u32 = 131116;
const ERR_VAULT_FUNGIBLE_MAX_AMOUNT_EXCEEDED: u32 = 131117;
const ERR_VAULT_ADD_FUNGIBLE_ASSET_MISMATCH: u32 = 131118;
const ERR_VAULT_NON_FUNGIBLE_ALREADY_EXISTED: u32 = 131119;
const ERR_VAULT_FUNGIBLE_AMOUNT_UNDERFLOW: u32 = 131120;
const ERR_VAULT_REMOVE_FUNGIBLE_ASSET_MISMATCH: u32 = 131121;
const ERR_VAULT_NON_FUNGIBLE_MISSING_ASSET: u32 = 131122;
const ERR_FUNGIBLE_ASSET_FORMAT_POSITION_ONE_MUST_BE_ZERO: u32 = 131123;
const ERR_ASSET_FORMAT_POSITION_TWO_MUST_BE_ZERO: u32 = 131124;
const ERR_FUNGIBLE_ASSET_FORMAT_POSITION_THREE_MUST_BE_ZERO: u32 = 131125;
const ERR_FUNGIBLE_ASSET_FORMAT_POSITION_ZERO_MUST_BE_ZERO: u32 = 131126;
const ERR_NON_FUNGIBLE_ASSET_FORMAT_POSITION_ONE_MUST_FUNGIBLE: u32 = 131127;
const ERR_NON_FUNGIBLE_ASSET_HIGH_BIT_SET: u32 = 131128;
const ERR_FUNGIBLE_ASSET_MISMATCH: u32 = 131129;
const ERR_NON_FUNGIBLE_ASSET_MISMATCH: u32 = 131130;
const ERR_ACCOUNT_NONCE_INCR_MUST_BE_U32: u32 = 131131;
const ERR_ACCOUNT_INSUFFICIENT_ONES: u32 = 131132;
const ERR_ACCOUNT_SET_CODE_ACCOUNT_MUST_BE_UPDATABLE: u32 = 131133;
const ERR_ACCOUNT_SEED_DIGEST_MISMATCH: u32 = 131134;
const ERR_ACCOUNT_INVALID_POW: u32 = 131135;
const ERR_NOTE_DATA_MISMATCH: u32 = 131136;
const ERR_ASSET_NOT_FUNGIBLE_ID: u32 = 131137;
const ERR_ASSET_INVALID_AMOUNT: u32 = 131138;
const ERR_ASSET_NOT_NON_FUNGIBLE_ID: u32 = 131139;
const ERR_INVALID_NOTE_TYPE: u32 = 131140;
const ERR_NOTE_INVALID_TAG_PREFIX_FOR_TYPE: u32 = 131141;
const ERR_NOTE_INVALID_TAG_HIGH_BIT_SET: u32 = 131142;

pub const KERNEL_ERRORS: [(u32, &str); 71] = [
    (ERR_FAUCET_RESERVED_DATA_SLOT, "For faucets the slot FAUCET_STORAGE_DATA_SLOT is reserved and can not be used with set_account_item"),
    (ERR_ACCT_MUST_BE_A_FAUCET, "Procedure can only be called for faucet accounts"),
    (ERR_P2ID_WRONG_NUMBER_OF_INPUTS, "P2ID scripts expect exactly 1 note input"),
    (ERR_P2ID_TARGET_ACCT_MISMATCH, "P2ID's target account address and transaction address do no match"),
    (ERR_P2IDR_WRONG_NUMBER_OF_INPUTS, "P2IDR scripts expect exactly 2 note inputs"),
    (ERR_P2IDR_RECLAIM_ACCT_IS_NOT_SENDER, "P2IDR's can only be reclaimed by the sender"),
    (ERR_P2IDR_RECLAIM_HEIGHT_NOT_REACHED, "Transaction's reference block is lower than reclaim height. The P2IDR can not be reclaimed"),
    (ERR_SWAP_WRONG_NUMBER_OF_INPUTS, "SWAP script expects exactly 9 note inputs"),
    (ERR_SWAP_WRONG_NUMBER_OF_ASSETS, "SWAP script requires exactly one note asset"),
    (ERR_KERNEL_TX_NONCE_DID_NOT_INCREASE, "The nonce did not increase after a state changing transaction"),
    (ERR_KERNEL_ASSET_MISMATCH, "Total assets at the transaction end must match"),
    (ERR_PROLOGUE_GLOBAL_INPUTS_MISMATCH, "The global inputs provided via the advice provider do not match the block hash commitment"),
    (ERR_PROLOGUE_ACCT_STORAGE_MISMATCH, "The account storage data provided via the advice provider do not match its state commitment"),
    (ERR_PROLOGUE_ACCT_STORAGE_ARITY_TOO_HIGH, "Data store in account's storage exceeds the maximum capacity of 256 elements"),
    (ERR_PROLOGUE_ACCT_STORAGE_TYPE_INVALID, "Data store in account's storage contains invalid type discriminant"),
    (ERR_PROLOGUE_NEW_ACCT_VAULT_NOT_EMPTY, "New account must start with an empty vault"),
    (ERR_PROLOGUE_NEW_ACCT_INVALID_SLOT_TYPE, "New account must have valid slot type s"),
    (ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_NON_EMPTY_RESERVED_SLOT, "Fungible faucet reserved slot must start empty"),
    (ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_NON_ZERO_RESERVED_SLOT, "Fungible faucet reserved slot must start with zero arity"),
    (ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_INVALID_TYPE_RESERVED_SLOT, "Fungible faucet reserved slot must start with no type"),
    (ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_INVALID_RESERVED_SLOT, "Non-fungible faucet reserved slot must start as an empty SMT"),
    (ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_NON_ZERO_RESERVED_SLOT, "Non-fungible faucet reserved slot must start with zero arity"),
    (ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_INVALID_TYPE_RESERVED_SLOT, "Non-fungible faucet reserved slot must start with no type"),
    (ERR_PROLOGUE_ACCT_HASH_MISMATCH, "The account data provided via advice provider did not match the initial hash"),
    (ERR_PROLOGUE_OLD_ACCT_NONCE_ZERO, "Existing account must not have a zero nonce"),
    (ERR_PROLOGUE_ACCT_ID_MISMATCH, "Account id and global account id must match"),
    (ERR_PROLOGUE_NOTE_MMR_DIGEST_MISMATCH, "Reference block MMR and note's authentication MMR must match"),
    (ERR_PROLOGUE_NOTE_TOO_MANY_INPUTS, "Note with too many inputs"),
    (ERR_PROLOGUE_NOTE_TOO_MANY_ASSETS, "Note with too many assets"),
    (ERR_PROLOGUE_NOTE_CONSUMED_ASSETS_MISMATCH, "Note's consumed assets provided via advice provider mistmatch its commitment"),
    (ERR_PROLOGUE_TOO_MANY_INPUT_NOTES, "Number of input notes can no exceed the kernel's maximum limit"),
    (ERR_PROLOGUE_INPUT_NOTES_NULLIFIER_COMMITMENT_MISMATCH, "Input notes nullifier commitment did not match the provided data"),
    (ERR_TX_OUTPUT_NOTES_OVERFLOW, "Output notes exceeded the maximum limit"),
    (ERR_BASIC_FUNGIBLE_MAX_SUPPLY_OVERFLOW, "Distribute would cause the max supply to be exceeded"),
    (ERR_FAUCET_ISSUANCE_OVERFLOW, "Asset mint operation would acuse a issuance overflow"),
    (ERR_FAUCET_BURN_OVER_ISSUANCE, "Asset burn can not exceed the existing supply"),
    (ERR_FAUCET_NON_FUNGIBLE_ALREADY_EXISTS, "Non fungible token already exists, it can be issue only once"),
    (ERR_FAUCET_NON_FUNGIBLE_BURN_WRONG_TYPE, "Non fungible burn called on the wrong faucet type"),
    (ERR_FAUCET_NONEXISTING_TOKEN, "Non fungible burn called on inexisting token"),
    (ERR_NOTE_INVALID_SENDER, "Input note can not have an empty sender, procedure was likely called from the wrong context"),
    (ERR_NOTE_INVALID_VAULT, "Input note can not have an empty vault, procedure was likely called from the wrong context"),
    (ERR_NOTE_INVALID_INPUTS, "Input note can not have empty inputs, procedure was likely called from the wrong context"),
    (ERR_NOTE_TOO_MANY_ASSETS, "Note's asset must fit in a u32"),
    (ERR_VAULT_GET_BALANCE_WRONG_ASSET_TYPE, "The get_balance procedure can be called only with a fungible faucet"),
    (ERR_VAULT_HAS_NON_FUNGIBLE_WRONG_ACCOUNT_TYPE, "The has_non_fungible_asset procedure can be called only with a non-fungible faucet"),
    (ERR_VAULT_FUNGIBLE_MAX_AMOUNT_EXCEEDED, "Adding the fungible asset would exceed the max_amount"),
    (ERR_VAULT_ADD_FUNGIBLE_ASSET_MISMATCH, "Decorator value did not match the assert commitment"),
    (ERR_VAULT_NON_FUNGIBLE_ALREADY_EXISTED, "The non-fungible asset already existed, can not be added again"),
    (ERR_VAULT_FUNGIBLE_AMOUNT_UNDERFLOW, "Removing the fungible asset would have current amount being negative"),
    (ERR_VAULT_REMOVE_FUNGIBLE_ASSET_MISMATCH, "Data provided via decorator did not match the commitment"),
    (ERR_VAULT_NON_FUNGIBLE_MISSING_ASSET, "Removing inexisting non-fungible asset"),
    (ERR_FUNGIBLE_ASSET_FORMAT_POSITION_ONE_MUST_BE_ZERO, "The felt at position 1 must be zero"),
    (ERR_ASSET_FORMAT_POSITION_TWO_MUST_BE_ZERO, "The felt at position 2 must be zero"),
    (ERR_FUNGIBLE_ASSET_FORMAT_POSITION_THREE_MUST_BE_ZERO, "The felt at position 3 must correspond to a fungible"),
    (ERR_FUNGIBLE_ASSET_FORMAT_POSITION_ZERO_MUST_BE_ZERO, "The felt at position 0 must be within limit"),
    (ERR_NON_FUNGIBLE_ASSET_FORMAT_POSITION_ONE_MUST_FUNGIBLE, "The felt at position 1 must be zero"),
    (ERR_NON_FUNGIBLE_ASSET_HIGH_BIT_SET, "The felt at position 3 must be zero"),
    (ERR_FUNGIBLE_ASSET_MISMATCH, "Fungible asset origin validation failed"),
    (ERR_NON_FUNGIBLE_ASSET_MISMATCH, "Non-fungible asset origin validation failed"),
    (ERR_ACCOUNT_NONCE_INCR_MUST_BE_U32, "The nonce increase must be a u32"),
    (ERR_ACCOUNT_INSUFFICIENT_ONES, "Account id format is invalid, insufficient ones"),
    (ERR_ACCOUNT_SET_CODE_ACCOUNT_MUST_BE_UPDATABLE, "Account must be updatable for it to be possible to update its code"),
    (ERR_ACCOUNT_SEED_DIGEST_MISMATCH, "Account seed digest mismatch"),
    (ERR_ACCOUNT_INVALID_POW, "Account pow is insufficient"),
    (ERR_NOTE_DATA_MISMATCH, "Note's advice data does not match the expected commitment"),
    (ERR_ASSET_NOT_FUNGIBLE_ID, "Can not build the fungible asset because provided id is not a fungible id"),
    (ERR_ASSET_INVALID_AMOUNT, "Can not build the asset because amount exceeds the maximum"),
    (ERR_ASSET_NOT_NON_FUNGIBLE_ID, "Can not build the non-fungible asset because provided id is not a non-fungible id"),
    (ERR_INVALID_NOTE_TYPE, "Invalid note type"),
    (ERR_NOTE_INVALID_TAG_PREFIX_FOR_TYPE, "The note's tag failed the most significant validation"),
    (ERR_NOTE_INVALID_TAG_HIGH_BIT_SET, "The note's tag high bits must be set to zero"),
];
