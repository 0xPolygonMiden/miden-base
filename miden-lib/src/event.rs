use miden_objects::Word;
use vm_processor::ExecutionError;

#[repr(u32)]
pub enum Event {
    AddAssetToAccountVault = 0,
    RemoveAssetFromAccountVault = 1,
}

impl TryFrom<u32> for Event {
    type Error = ExecutionError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Event::AddAssetToAccountVault),
            1 => Ok(Event::RemoveAssetFromAccountVault),
            // TODO: Change to correct error
            _ => Err(ExecutionError::AdviceMapKeyNotFound(Word::default())),
        }
    }
}
