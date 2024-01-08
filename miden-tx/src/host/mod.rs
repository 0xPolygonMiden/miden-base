use miden_lib::transaction::TransactionEvent;
use miden_objects::{accounts::delta::AccountVaultDelta, utils::string::ToString};
use vm_processor::{
    AdviceExtractor, AdviceInjector, AdviceProvider, ContextId, ExecutionError, Host, HostResponse,
    ProcessState,
};

mod account_delta;
use account_delta::AccountVaultDeltaTracker;

// TRANSACTION HOST
// ================================================================================================

/// Transaction host is responsible for handling [Host] requests made by a transaction kernel.
///
/// Transaction host is composed of two components:
/// - An advice provider which is used to provide non-deterministic inputs to the transaction
///   runtime.
/// - An account vault delta tracker which is used to keep track of changes made to the asset
///   of the account the transaction is being executed against.
pub struct TransactionHost<A> {
    adv_provider: A,
    acct_vault_delta_tracker: AccountVaultDeltaTracker,
}

impl<A: AdviceProvider> TransactionHost<A> {
    /// Returns a new [TransactionHost] instance with the provided [AdviceProvider].
    pub fn new(adv_provider: A) -> Self {
        Self {
            adv_provider,
            acct_vault_delta_tracker: AccountVaultDeltaTracker::default(),
        }
    }

    /// Consumes this transaction host and returns the advice provider and account vault delta.
    pub fn into_parts(self) -> (A, AccountVaultDelta) {
        (self.adv_provider, self.acct_vault_delta_tracker.into_vault_delta())
    }
}

impl<A: AdviceProvider> Host for TransactionHost<A> {
    fn get_advice<S: ProcessState>(
        &mut self,
        process: &S,
        extractor: AdviceExtractor,
    ) -> Result<HostResponse, ExecutionError> {
        self.adv_provider.get_advice(process, &extractor)
    }

    fn set_advice<S: ProcessState>(
        &mut self,
        process: &S,
        injector: AdviceInjector,
    ) -> Result<HostResponse, ExecutionError> {
        self.adv_provider.set_advice(process, &injector)
    }

    fn on_event<S: ProcessState>(
        &mut self,
        process: &S,
        event_id: u32,
    ) -> Result<HostResponse, ExecutionError> {
        let event = TransactionEvent::try_from(event_id)
            .map_err(|err| ExecutionError::EventError(err.to_string()))?;

        if process.ctx() != ContextId::root() {
            return Err(ExecutionError::EventError(format!(
                "{event} event can only be emitted from the root context"
            )));
        }

        use TransactionEvent::*;
        match event {
            AddAssetToAccountVault => self.acct_vault_delta_tracker.add_asset(process),
            RemoveAssetFromAccountVault => self.acct_vault_delta_tracker.remove_asset(process),
        }
    }
}
