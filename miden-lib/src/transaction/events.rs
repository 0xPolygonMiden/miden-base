use vm_processor::ExecutionError;

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
pub enum TransactionEvent {
    AddAssetToAccountVault = 0x2_0000,      // 131072
    RemoveAssetFromAccountVault = 0x2_0001, // 131073
}

impl TryFrom<u32> for TransactionEvent {
    type Error = ExecutionError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0x2_0000 => Ok(TransactionEvent::AddAssetToAccountVault),
            0x2_0001 => Ok(TransactionEvent::RemoveAssetFromAccountVault),
            _ => Err(ExecutionError::EventError(format!(
                "Failed to parse Event - event with id {value} is not supported",
            ))),
        }
    }
}
