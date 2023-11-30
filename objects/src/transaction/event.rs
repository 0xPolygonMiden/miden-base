use vm_processor::ExecutionError;

/// Represents an event which is emitted by a transaction via the invocation of the
/// `emit.<event_id>` instruction. The event ID is a 32-bit unsigned integer which is used to
/// identify the event type.
#[repr(u32)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Event {
    AddAssetToAccountVault = 131072,
    RemoveAssetFromAccountVault = 131073,
}

impl TryFrom<u32> for Event {
    type Error = ExecutionError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            131072 => Ok(Event::AddAssetToAccountVault),
            131073 => Ok(Event::RemoveAssetFromAccountVault),
            _ => Err(ExecutionError::EventError(format!(
                "Failed to parse Event - event with id {value} is not supported",
            ))),
        }
    }
}
