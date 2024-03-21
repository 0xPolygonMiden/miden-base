use alloc::string::ToString;

use miden_lib::transaction::TransactionEvent;
use miden_objects::{
    accounts::{delta::AccountVaultDelta, AccountStub},
    Digest,
};
use vm_processor::{
    crypto::NodeIndex, AdviceExtractor, AdviceInjector, AdviceInputs, AdviceProvider, AdviceSource,
    ContextId, ExecutionError, Host, HostResponse, MemAdviceProvider, ProcessState,
};

mod account_procs;
use account_procs::AccountProcedureIndexMap;

// MOCK HOST
// ================================================================================================

/// This is very similar to the TransactionHost in miden-tx. The differences include:
/// - We do not track account delta here.
/// - There is special handling of EMPTY_DIGEST in account procedure index map.
/// - This host uses `MemAdviceProvider` which is instantiated from the passed in advice inputs.
pub struct MockHost {
    adv_provider: MemAdviceProvider,
    acct_procedure_index_map: AccountProcedureIndexMap,
}

impl MockHost {
    /// Returns a new [MockHost] instance with the provided [AdviceInputs].
    pub fn new(account: AccountStub, advice_inputs: AdviceInputs) -> Self {
        let adv_provider: MemAdviceProvider = advice_inputs.into();
        let proc_index_map = AccountProcedureIndexMap::new(account.code_root(), &adv_provider);
        Self {
            adv_provider,
            acct_procedure_index_map: proc_index_map,
        }
    }

    /// Consumes `self` and returns the advice provider and account vault delta.
    pub fn into_parts(self) -> (MemAdviceProvider, AccountVaultDelta) {
        (self.adv_provider, AccountVaultDelta::default())
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

impl Host for MockHost {
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
            AccountPushProcedureIndex => self.on_push_account_procedure_index(process),
            _ => Ok(()),
        }?;

        Ok(HostResponse::None)
    }
}
