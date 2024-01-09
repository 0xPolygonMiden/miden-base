use core::fmt;

use super::TransactionEventParsingError;

// TRANSACTION EVENT
// ================================================================================================

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
    AccountVaultAddAsset = 0x2_0000,      // 131072
    AccountVaultRemoveAsset = 0x2_0001,   // 131073
    AccountStorageSetItem = 0x2_0002,     // 131074
    AccountIncrementNonce = 0x2_0003,     // 131075
    AccountPushProcedureIndex = 0x2_0004, // 131076
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
            0x2_0000 => Ok(TransactionEvent::AccountVaultAddAsset),
            0x2_0001 => Ok(TransactionEvent::AccountVaultRemoveAsset),
            0x2_0002 => Ok(TransactionEvent::AccountStorageSetItem),
            0x2_0003 => Ok(TransactionEvent::AccountIncrementNonce),
            0x2_0004 => Ok(TransactionEvent::AccountPushProcedureIndex),
            _ => Err(TransactionEventParsingError::InvalidTransactionEvent(value)),
        }
    }
}
