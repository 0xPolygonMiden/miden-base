use miden_objects::{accounts::AccountVaultDelta, transaction::Event};
use vm_processor::{ExecutionError, HostResponse, ProcessState};

mod vault_delta;
use vault_delta::AccountVaultDeltaHandler;

/// The [EventHandler] is responsible for handling events emitted by a transaction.
///
/// It is composed of multiple sub-handlers, each of which is responsible for handling specific
/// event types. The event handler has access to the [ProcessState] at the time the event was
/// emitted, and can use it to extract the data required to handle the event.
///
/// Below we define the sub-handlers and their associated event types:
///
/// - [VaultDeltaHandler]:
///    - [Event::AddAssetToAccountVault]
///    - [Event::RemoveAssetFromAccountVault]
#[derive(Default, Debug)]
pub struct EventHandler {
    acct_vault_delta_handler: AccountVaultDeltaHandler,
}

impl EventHandler {
    /// Handles the event with the provided event ID.
    pub fn handle_event<S: ProcessState>(
        &mut self,
        process: &S,
        event_id: u32,
    ) -> Result<HostResponse, ExecutionError> {
        match Event::try_from(event_id)? {
            Event::AddAssetToAccountVault => self.acct_vault_delta_handler.add_asset(process),
            Event::RemoveAssetFromAccountVault => {
                self.acct_vault_delta_handler.remove_asset(process)
            },
        }
    }

    /// Consumes the [EventHandler] and finalizes the sub-handlers it is composed of.
    ///
    /// Returns the result of finalizing the sub-handlers.
    pub fn finalize(self) -> AccountVaultDelta {
        self.acct_vault_delta_handler.finalize()
    }
}
