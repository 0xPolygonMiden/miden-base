use alloc::{string::String, vec::Vec};
use core::fmt;

use miden_objects::{
    accounts::AccountStorage, notes::NoteMetadata, AccountError, AssetError, Digest, Felt,
    NoteError,
};

// TRANSACTION KERNEL ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TransactionKernelError {
    FailedToAddAssetToNote(NoteError),
    InvalidNoteInputs {
        expected: Digest,
        got: Digest,
        data: Option<Vec<Felt>>,
    },
    InvalidStorageSlotIndex(u64),
    MalformedAccountId(AccountError),
    MalformedAsset(AssetError),
    MalformedAssetOnAccountVaultUpdate(AssetError),
    MalformedNoteInputs(NoteError),
    MalformedNoteMetadata(NoteError),
    MalformedNoteIndex(u64),
    MalformedNoteScript(Vec<Felt>),
    MalformedNoteType(NoteError),
    MalformedRecipientData(Vec<Felt>),
    MalformedTag(Felt),
    MissingNote(String),
    MissingNoteDetails(NoteMetadata, Digest),
    MissingNoteInputs,
    MissingStorageSlotValue(u8, String),
    TooFewElementsForNoteInputs,
    UnknownAccountProcedure(Digest),
}

impl fmt::Display for TransactionKernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionKernelError::FailedToAddAssetToNote(err) => {
                write!(f, "failed to add asset to note: {err}")
            },
            TransactionKernelError::InvalidNoteInputs { expected, got, data } => {
                write!(
                    f,
                    "The note input data does not match its hash, expected: {} got: {} data {:?}",
                    expected, got, data
                )
            },
            TransactionKernelError::InvalidStorageSlotIndex(index) => {
                let num_slots = AccountStorage::NUM_STORAGE_SLOTS;
                write!(f, "storage slot index {index} is invalid, must be smaller than {num_slots}")
            },
            TransactionKernelError::MalformedAccountId(err) => {
                write!( f, "Account id data extracted from the stack by the event handler is not well formed {err}")
            },
            TransactionKernelError::MalformedAsset(err) => {
                write!(f, "Asset data extracted from the stack by the event handler is not well formed {err}")
            },
            TransactionKernelError::MalformedAssetOnAccountVaultUpdate(err) => {
                write!(f, "malformed asset during account vault update: {err}")
            },
            TransactionKernelError::MalformedNoteInputs(err) => {
                write!( f, "Note inputs data extracted from the advice map by the event handler is not well formed {err}")
            },
            TransactionKernelError::MalformedNoteMetadata(err) => {
                write!(f, "Note metadata created by the event handler is not well formed {err}")
            },
            TransactionKernelError::MalformedNoteIndex(err) => {
                write!(f, "Note index is malformed {err}")
            },
            TransactionKernelError::MalformedNoteScript(data) => {
                write!( f, "Note script data extracted from the advice map by the event handler is not well formed {data:?}")
            },
            TransactionKernelError::MalformedNoteType(err) => {
                write!( f, "Note type data extracted from the stack by the event handler is not well formed {err}")
            },
            TransactionKernelError::MalformedRecipientData(data) => {
                write!(f, "Recipient data in the advice provider is not well formed {data:?}")
            },
            TransactionKernelError::MalformedTag(tag) => {
                write!( f, "Tag data extracted from the stack by the event handler is not well formed {tag}")
            },
            TransactionKernelError::MissingNote(note_idx) => {
                write!(f, "Cannot add asset to note with index {note_idx}, note does not exist in the advice provider")
            },
            TransactionKernelError::MissingNoteDetails(metadata, recipient) => {
                write!( f, "Public note missing the details in the advice provider. metadata: {metadata:?}, recipient: {recipient:?}")
            },
            TransactionKernelError::MissingNoteInputs => {
                write!(f, "Public note missing or incomplete inputs in the advice provider")
            },
            TransactionKernelError::MissingStorageSlotValue(index, err) => {
                write!(f, "value for storage slot {index} could not be found: {err}")
            },
            TransactionKernelError::TooFewElementsForNoteInputs => {
                write!(
                    f,
                    "note input data in advice provider contains fewer elements than specified by its inputs length"
                )
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
