use core::fmt;

use super::{TransactionEventParsingError, TransactionTraceParsingError};

// CONSTANTS
// ================================================================================================

/// Value of the top 16 bits of a transaction kernel event ID.
pub const EVENT_ID_PREFIX: u32 = 2;

// TRANSACTION EVENT
// ================================================================================================

const ACCOUNT_VAULT_ADD_ASSET: u32 = 0x2_0000; // 131072
const ACCOUNT_VAULT_REMOVE_ASSET: u32 = 0x2_0001; // 131073
const ACCOUNT_STORAGE_SET_ITEM: u32 = 0x2_0002; // 131074
const ACCOUNT_INCREMENT_NONCE: u32 = 0x2_0003; // 131075
const ACCOUNT_PUSH_PROCEDURE_INDEX: u32 = 0x2_0004; // 131076
const NOTE_CREATED: u32 = 0x2_0005; // 131077

/// Events which may be emitted by a transaction kernel.
///
/// The events are emitted via the `emit.<event_id>` instruction. The event ID is a 32-bit
/// unsigned integer which is used to identify the event type. For events emitted by the
/// transaction kernel, the event_id is structured as follows:
/// - The upper 16 bits of the event ID are set to 2.
/// - The lower 16 bits represent a unique event ID within the transaction kernel.
#[repr(u32)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TransactionEvent {
    AccountVaultAddAsset = ACCOUNT_VAULT_ADD_ASSET,
    AccountVaultRemoveAsset = ACCOUNT_VAULT_REMOVE_ASSET,
    AccountStorageSetItem = ACCOUNT_STORAGE_SET_ITEM,
    AccountIncrementNonce = ACCOUNT_INCREMENT_NONCE,
    AccountPushProcedureIndex = ACCOUNT_PUSH_PROCEDURE_INDEX,
    NoteCreated = NOTE_CREATED,
}

impl fmt::Display for TransactionEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl TryFrom<u32> for TransactionEvent {
    type Error = TransactionEventParsingError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value >> 16 != EVENT_ID_PREFIX {
            return Err(TransactionEventParsingError::NotTransactionEvent(value));
        }

        match value {
            ACCOUNT_VAULT_ADD_ASSET => Ok(TransactionEvent::AccountVaultAddAsset),
            ACCOUNT_VAULT_REMOVE_ASSET => Ok(TransactionEvent::AccountVaultRemoveAsset),
            ACCOUNT_STORAGE_SET_ITEM => Ok(TransactionEvent::AccountStorageSetItem),
            ACCOUNT_INCREMENT_NONCE => Ok(TransactionEvent::AccountIncrementNonce),
            ACCOUNT_PUSH_PROCEDURE_INDEX => Ok(TransactionEvent::AccountPushProcedureIndex),
            NOTE_CREATED => Ok(TransactionEvent::NoteCreated),
            _ => Err(TransactionEventParsingError::InvalidTransactionEvent(value)),
        }
    }
}

// TRANSACTION TRACE
// ================================================================================================

#[repr(u32)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TransactionTrace {
    PrologueStart = 0x2_0000,           // 131072
    PrologueEnd = 0x2_0001,             // 131073
    NotesProcessingStart = 0x2_0002,    // 131074
    NotesProcessingEnd = 0x2_0003,      // 131075
    NoteExecutionStart = 0x2_0004,      // 131076
    NoteExecutionEnd = 0x2_0005,        // 131077
    TxScriptProcessingStart = 0x2_0006, // 131078
    TxScriptProcessingEnd = 0x2_0007,   // 131079
    EpilogueStart = 0x2_0008,           // 131080
    EpilogueEnd = 0x2_0009,             // 131081
}

impl fmt::Display for TransactionTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl TryFrom<u32> for TransactionTrace {
    type Error = TransactionTraceParsingError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value >> 16 != EVENT_ID_PREFIX {
            return Err(TransactionTraceParsingError::NotTransactionTrace(value));
        }

        match value {
            0x2_0000 => Ok(TransactionTrace::PrologueStart),
            0x2_0001 => Ok(TransactionTrace::PrologueEnd),
            0x2_0002 => Ok(TransactionTrace::NotesProcessingStart),
            0x2_0003 => Ok(TransactionTrace::NotesProcessingEnd),
            0x2_0004 => Ok(TransactionTrace::NoteExecutionStart),
            0x2_0005 => Ok(TransactionTrace::NoteExecutionEnd),
            0x2_0006 => Ok(TransactionTrace::TxScriptProcessingStart),
            0x2_0007 => Ok(TransactionTrace::TxScriptProcessingEnd),
            0x2_0008 => Ok(TransactionTrace::EpilogueStart),
            0x2_0009 => Ok(TransactionTrace::EpilogueEnd),
            _ => Err(TransactionTraceParsingError::InvalidTransactionTrace(value)),
        }
    }
}
