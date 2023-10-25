use miden_lib::event::Event;
use vm_processor::{ExecutionError, HostResponse, ProcessState};

mod vault_delta;
use vault_delta::VaultDeltaHandler;

#[derive(Default, Debug)]
pub struct EventHandler {
    vault_delta_handler: VaultDeltaHandler,
}

impl EventHandler {
    pub fn handle_event<S: ProcessState>(
        &mut self,
        process: &S,
        event_id: u32,
    ) -> Result<HostResponse, ExecutionError> {
        match Event::try_from(event_id)? {
            Event::AddAssetToAccountVault => self.vault_delta_handler.add_asset(process),
            Event::RemoveAssetFromAccountVault => self.vault_delta_handler.remove_asset(process),
        }
    }
}
