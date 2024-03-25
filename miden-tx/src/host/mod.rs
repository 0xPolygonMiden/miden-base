use alloc::{collections::BTreeMap, string::ToString};

use miden_lib::transaction::{TransactionEvent, TransactionKernelError};
use miden_objects::{
    accounts::{AccountDelta, AccountStub},
    Digest,
};
use vm_processor::{
    crypto::NodeIndex, AdviceExtractor, AdviceInjector, AdviceProvider, AdviceSource, ContextId,
    ExecutionError, Host, HostResponse, ProcessState,
};

mod account_delta;
use account_delta::AccountDeltaTracker;

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
    account_delta: AccountDeltaTracker,
    acct_procedure_index_map: AccountProcedureIndexMap,
}

impl<A: AdviceProvider> TransactionHost<A> {
    /// Returns a new [TransactionHost] instance with the provided [AdviceProvider].
    pub fn new(account: AccountStub, adv_provider: A) -> Self {
        let proc_index_map = AccountProcedureIndexMap::new(account.code_root(), &adv_provider);
        Self {
            adv_provider,
            account_delta: AccountDeltaTracker::new(&account),
            acct_procedure_index_map: proc_index_map,
        }
    }

    /// Consumes `self` and returns the advice provider and account vault delta.
    pub fn into_parts(self) -> (A, AccountDelta) {
        (self.adv_provider, self.account_delta.into_delta())
    }

    // EVENT HANDLERS
    // --------------------------------------------------------------------------------------------

    fn on_account_push_procedure_index<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        let proc_idx = self.acct_procedure_index_map.get_proc_index(process)?;
        self.adv_provider
            .push_stack(AdviceSource::Value(proc_idx.into()))
            .expect("failed to push value onto advice stack");
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
            AccountVaultAddAsset => self.on_account_vault_add_asset(process),
            AccountVaultRemoveAsset => self.on_account_vault_remove_asset(process),
            AccountStorageSetItem => self.on_account_storage_set_item(process),
            AccountIncrementNonce => self.on_account_increment_nonce(process),
            AccountPushProcedureIndex => self.on_account_push_procedure_index(process),
        }
        .map_err(|err| ExecutionError::EventError(err.to_string()))?;

        Ok(HostResponse::None)
    }
}
