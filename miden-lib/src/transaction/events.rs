use core::fmt;

use super::{TransactionEventError, TransactionTraceParsingError};

// CONSTANTS
// ================================================================================================

/// Value of the top 16 bits of a transaction kernel event ID.
pub const EVENT_ID_PREFIX: u32 = 2;

// TRANSACTION EVENT
// ================================================================================================

const ACCOUNT_VAULT_BEFORE_ADD_ASSET: u32 = 0x2_0000; // 131072
const ACCOUNT_VAULT_AFTER_ADD_ASSET: u32 = 0x2_0001; // 131073

const ACCOUNT_VAULT_BEFORE_REMOVE_ASSET: u32 = 0x2_0002; // 131074
const ACCOUNT_VAULT_AFTER_REMOVE_ASSET: u32 = 0x2_0003; // 131075

const ACCOUNT_STORAGE_BEFORE_SET_ITEM: u32 = 0x2_0004; // 131076
const ACCOUNT_STORAGE_AFTER_SET_ITEM: u32 = 0x2_0005; // 131077

const ACCOUNT_STORAGE_BEFORE_SET_MAP_ITEM: u32 = 0x2_0006; // 131078
const ACCOUNT_STORAGE_AFTER_SET_MAP_ITEM: u32 = 0x2_0007; // 131079

const ACCOUNT_BEFORE_INCREMENT_NONCE: u32 = 0x2_0008; // 131080
const ACCOUNT_AFTER_INCREMENT_NONCE: u32 = 0x2_0009; // 131081

const ACCOUNT_PUSH_PROCEDURE_INDEX: u32 = 0x2_000a; // 131082

const NOTE_BEFORE_CREATED: u32 = 0x2_000b; // 131083
const NOTE_AFTER_CREATED: u32 = 0x2_000c; // 131084

const NOTE_BEFORE_ADD_ASSET: u32 = 0x2_000d; // 131085
const NOTE_AFTER_ADD_ASSET: u32 = 0x2_000e; // 131086

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
    AccountVaultBeforeAddAsset = ACCOUNT_VAULT_BEFORE_ADD_ASSET,
    AccountVaultAfterAddAsset = ACCOUNT_VAULT_AFTER_ADD_ASSET,

    AccountVaultBeforeRemoveAsset = ACCOUNT_VAULT_BEFORE_REMOVE_ASSET,
    AccountVaultAfterRemoveAsset = ACCOUNT_VAULT_AFTER_REMOVE_ASSET,

    AccountStorageBeforeSetItem = ACCOUNT_STORAGE_BEFORE_SET_ITEM,
    AccountStorageAfterSetItem = ACCOUNT_STORAGE_AFTER_SET_ITEM,

    AccountStorageBeforeSetMapItem = ACCOUNT_STORAGE_BEFORE_SET_MAP_ITEM,
    AccountStorageAfterSetMapItem = ACCOUNT_STORAGE_AFTER_SET_MAP_ITEM,

    AccountBeforeIncrementNonce = ACCOUNT_BEFORE_INCREMENT_NONCE,
    AccountAfterIncrementNonce = ACCOUNT_AFTER_INCREMENT_NONCE,

    AccountPushProcedureIndex = ACCOUNT_PUSH_PROCEDURE_INDEX,

    NoteBeforeCreated = NOTE_BEFORE_CREATED,
    NoteAfterCreated = NOTE_AFTER_CREATED,

    NoteBeforeAddAsset = NOTE_BEFORE_ADD_ASSET,
    NoteAfterAddAsset = NOTE_AFTER_ADD_ASSET,
}

impl fmt::Display for TransactionEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl TryFrom<u32> for TransactionEvent {
    type Error = TransactionEventError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value >> 16 != EVENT_ID_PREFIX {
            return Err(TransactionEventError::NotTransactionEvent(value));
        }

        match value {
            ACCOUNT_VAULT_BEFORE_ADD_ASSET => Ok(TransactionEvent::AccountVaultBeforeAddAsset),
            ACCOUNT_VAULT_AFTER_ADD_ASSET => Ok(TransactionEvent::AccountVaultAfterAddAsset),

            ACCOUNT_VAULT_BEFORE_REMOVE_ASSET => {
                Ok(TransactionEvent::AccountVaultBeforeRemoveAsset)
            },
            ACCOUNT_VAULT_AFTER_REMOVE_ASSET => Ok(TransactionEvent::AccountVaultAfterRemoveAsset),

            ACCOUNT_STORAGE_BEFORE_SET_ITEM => Ok(TransactionEvent::AccountStorageBeforeSetItem),
            ACCOUNT_STORAGE_AFTER_SET_ITEM => Ok(TransactionEvent::AccountStorageAfterSetItem),

            ACCOUNT_STORAGE_BEFORE_SET_MAP_ITEM => {
                Ok(TransactionEvent::AccountStorageBeforeSetMapItem)
            },
            ACCOUNT_STORAGE_AFTER_SET_MAP_ITEM => {
                Ok(TransactionEvent::AccountStorageAfterSetMapItem)
            },

            ACCOUNT_BEFORE_INCREMENT_NONCE => Ok(TransactionEvent::AccountBeforeIncrementNonce),
            ACCOUNT_AFTER_INCREMENT_NONCE => Ok(TransactionEvent::AccountAfterIncrementNonce),

            ACCOUNT_PUSH_PROCEDURE_INDEX => Ok(TransactionEvent::AccountPushProcedureIndex),

            NOTE_BEFORE_CREATED => Ok(TransactionEvent::NoteBeforeCreated),
            NOTE_AFTER_CREATED => Ok(TransactionEvent::NoteAfterCreated),

            NOTE_BEFORE_ADD_ASSET => Ok(TransactionEvent::NoteBeforeAddAsset),
            NOTE_AFTER_ADD_ASSET => Ok(TransactionEvent::NoteAfterAddAsset),

            _ => Err(TransactionEventError::InvalidTransactionEvent(value)),
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
            return Err(TransactionTraceParsingError::UnknownTransactionTrace(value));
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
            _ => Err(TransactionTraceParsingError::UnknownTransactionTrace(value)),
        }
    }
}
