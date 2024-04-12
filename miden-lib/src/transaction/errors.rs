use alloc::{string::String, vec::Vec};
use core::fmt;

use miden_objects::{
    accounts::AccountStorage,
    notes::{NoteAssets, NoteMetadata},
    AccountError, AssetError, Digest, Felt, NoteError,
};

// TRANSACTION KERNEL ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TransactionKernelError {
    InvalidStorageSlotIndex(u64),
    MalformedAccountId(AccountError),
    MalformedAsset(AssetError),
    MalformedAssetOnAccountVaultUpdate(AssetError),
    MalformedNoteInputs(NoteError),
    MalformedNoteMetadata(NoteError),
    MalformedNoteScript(Vec<Felt>),
    MalformedNoteType(NoteError),
    MalformedRecipientData(Vec<Felt>),
    MalformedTag(Felt),
    MissingNoteDetails(NoteMetadata, NoteAssets, Digest),
    MissingStorageSlotValue(u8, String),
    UnknownAccountProcedure(Digest),
}

impl fmt::Display for TransactionKernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionKernelError::InvalidStorageSlotIndex(index) => {
                let num_slots = AccountStorage::NUM_STORAGE_SLOTS;
                write!(f, "storage slot index {index} is invalid, must be smaller than {num_slots}")
            },
            TransactionKernelError::MalformedAccountId(err) => {
                write!( f, "Account id data extracted from the stack by the event handler is not well formed {}", err)
            },
            TransactionKernelError::MalformedAsset(err) => {
                write!(f, "Asset data extracted from the stack by the event handler is not well formed {:?}", err)
            },
            TransactionKernelError::MalformedAssetOnAccountVaultUpdate(err) => {
                write!(f, "malformed asset during account vault update: {err}")
            },
            TransactionKernelError::MalformedNoteInputs(err) => {
                write!( f, "Note inputs data extracted from the advice map by the event handler is not well formed {}", err)
            },
            TransactionKernelError::MalformedNoteMetadata(err) => {
                write!(f, "Note metadata created by the event handler is not well formed {:?}", err)
            },
            TransactionKernelError::MalformedNoteScript(data) => {
                write!( f, "Note script data extracted from the advice map by the event handler is not well formed {:?}", data)
            },
            TransactionKernelError::MalformedNoteType(err) => {
                write!( f, "Note type data extracted from the stack by the event handler is not well formed {}", err)
            },
            TransactionKernelError::MalformedRecipientData(data) => {
                write!(f, "Recipient data in the advice provider is not well formed {:?}", data)
            },
            TransactionKernelError::MalformedTag(tag) => {
                write!(
                    f,
                    "Tag data extracted from the stack by the event handler is not well formed {}",
                    tag
                )
            },
            TransactionKernelError::MissingNoteDetails(metadata, vault, recipient) => {
                write!( f, "Public note missing the details in the advice provider. metadata: {:?} vault: {:?} recipient: {:?}", metadata, vault, recipient)
            },
            TransactionKernelError::MissingStorageSlotValue(index, err) => {
                write!(f, "value for storage slot {index} could not be found: {err}")
            },
            TransactionKernelError::UnknownAccountProcedure(proc_root) => {
                write!(f, "account procedure with root {proc_root} is not in the advice provider")
            },
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionKernelError {}

// TRANSACTION EVENT PARSING ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TransactionEventParsingError {
    InvalidTransactionEvent(u32),
    NotTransactionEvent(u32),
}

impl fmt::Display for TransactionEventParsingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransactionEvent(event_id) => {
                write!(f, "event {event_id} is not a valid transaction kernel event")
            },
            Self::NotTransactionEvent(event_id) => {
                write!(f, "event {event_id} is not a transaction kernel event")
            },
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionEventParsingError {}

// TRANSACTION TRACE PARSING ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TransactionTraceParsingError {
    InvalidTransactionTrace(u32),
    NotTransactionTrace(u32),
}

impl fmt::Display for TransactionTraceParsingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransactionTrace(trace_id) => {
                write!(f, "trace {trace_id} is invalid")
            },
            Self::NotTransactionTrace(trace_id) => {
                write!(f, "trace {trace_id} is not a transaction kernel trace")
            },
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionTraceParsingError {}
