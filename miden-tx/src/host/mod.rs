use miden_lib::transaction::TransactionEvent;
use miden_objects::{
    accounts::{delta::AccountVaultDelta, AccountStub},
    utils::{collections::BTreeMap, string::ToString},
    Digest,
};
use vm_processor::{
    crypto::NodeIndex, AdviceExtractor, AdviceInjector, AdviceProvider, AdviceSource, ContextId,
    ExecutionError, Host, HostResponse, ProcessState,
};

mod account_delta;
use account_delta::AccountVaultDeltaTracker;

mod account_procs;
use account_procs::AccountProcedureIndexMap;

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
    acct_procedure_index_map: AccountProcedureIndexMap,
}

impl<A: AdviceProvider> TransactionHost<A> {
    /// Returns a new [TransactionHost] instance with the provided [AdviceProvider].
    pub fn new(account: AccountStub, adv_provider: A) -> Self {
        let proc_index_map = AccountProcedureIndexMap::new(account.code_root(), &adv_provider);
        Self {
            adv_provider,
            acct_vault_delta_tracker: AccountVaultDeltaTracker::default(),
            acct_procedure_index_map: proc_index_map,
        }
    }

    /// Consumes this transaction host and returns the advice provider and account vault delta.
    pub fn into_parts(self) -> (A, AccountVaultDelta) {
        (self.adv_provider, self.acct_vault_delta_tracker.into_vault_delta())
    }

    // EVENT HANDLERS
    // --------------------------------------------------------------------------------------------

    fn on_push_account_procedure_index<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), ExecutionError> {
        let proc_idx = self
            .acct_procedure_index_map
            .get_proc_index(process)
            .map_err(|err| ExecutionError::EventError(err.to_string()))?;
        self.adv_provider.push_stack(AdviceSource::Value(proc_idx.into()))?;
        Ok(())
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
            PushAccountProcedureIndex => self.on_push_account_procedure_index(process),
        }?;

        Ok(HostResponse::None)
    }
}
