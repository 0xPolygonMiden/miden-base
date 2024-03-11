use core::fmt;

use super::TransactionEventParsingError;

// TRANSACTION EVENT
// ================================================================================================

const ACCOUNT_VAULT_ADD_ASSET: u32 = 0x2_0000; // 131072
const ACCOUNT_VAULT_REMOVE_ASSET: u32 = 0x2_0001; // 131073
const ACCOUNT_STORAGE_SET_ITEM: u32 = 0x2_0002; // 131074
const ACCOUNT_INCREMENT_NONCE: u32 = 0x2_0003; // 131075
const ACCOUNT_PUSH_PROCEDURE_INDEX: u32 = 0x2_0004; // 131076
const NOTE_CREATED: u32 = 0x2_0005; // 131077
const ACCOUNT_STORAGE_SET_MAP_ITEM: u32 = 0x2_0006; // 131078

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
    AccountStorageSetMapItem = ACCOUNT_STORAGE_SET_MAP_ITEM,
}

impl TransactionEvent {
    /// Value of the top 16 bits of a transaction kernel event ID.
    pub const EVENT_ID_PREFIX: u16 = 2;
}

impl fmt::Display for TransactionEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl TryFrom<u32> for TransactionEvent {
    type Error = TransactionEventParsingError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value >> 16 != Self::EVENT_ID_PREFIX as u32 {
            return Err(TransactionEventParsingError::NotTransactionEvent(value));
        }

        match value {
            ACCOUNT_VAULT_ADD_ASSET => Ok(TransactionEvent::AccountVaultAddAsset),
            ACCOUNT_VAULT_REMOVE_ASSET => Ok(TransactionEvent::AccountVaultRemoveAsset),
            ACCOUNT_STORAGE_SET_ITEM => Ok(TransactionEvent::AccountStorageSetItem),
            ACCOUNT_INCREMENT_NONCE => Ok(TransactionEvent::AccountIncrementNonce),
            ACCOUNT_PUSH_PROCEDURE_INDEX => Ok(TransactionEvent::AccountPushProcedureIndex),
            NOTE_CREATED => Ok(TransactionEvent::NoteCreated),
            ACCOUNT_STORAGE_SET_MAP_ITEM => Ok(TransactionEvent::AccountStorageSetMapItem),
            _ => Err(TransactionEventParsingError::InvalidTransactionEvent(value)),
        }
    }
}
