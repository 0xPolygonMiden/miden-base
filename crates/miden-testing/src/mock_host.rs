use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    string::ToString,
    sync::Arc,
};

use miden_lib::{
    errors::tx_kernel_errors::TX_KERNEL_ERRORS,
    transaction::{TransactionEvent, TransactionEventError},
};
use miden_objects::{
    Digest,
    account::{AccountHeader, AccountVaultDelta},
};
use miden_tx::{TransactionMastStore, host::AccountProcedureIndexMap};
use vm_processor::{
    AdviceInputs, AdviceProvider, AdviceSource, ContextId, ExecutionError, Host, MastForest,
    MastForestStore, MemAdviceProvider, ProcessState,
};

// MOCK HOST
// ================================================================================================

/// This is very similar to the TransactionHost in miden-tx. The differences include:
/// - We do not track account delta here.
/// - There is special handling of EMPTY_DIGEST in account procedure index map.
/// - This host uses `MemAdviceProvider` which is instantiated from the passed in advice inputs.
pub struct MockHost {
    adv_provider: MemAdviceProvider,
    acct_procedure_index_map: AccountProcedureIndexMap,
    mast_store: Rc<TransactionMastStore>,
    /// Contains mappings from error codes to the related error messages.
    ///
    /// This map is initialized at construction time from the [`TX_KERNEL_ERRORS`] array.
    error_messages: BTreeMap<u32, &'static str>,
}

impl MockHost {
    /// Returns a new [MockHost] instance with the provided [AdviceInputs].
    pub fn new(
        account: AccountHeader,
        advice_inputs: AdviceInputs,
        mast_store: Rc<TransactionMastStore>,
        mut foreign_code_commitments: BTreeSet<Digest>,
    ) -> Self {
        foreign_code_commitments.insert(account.code_commitment());
        let adv_provider: MemAdviceProvider = advice_inputs.into();
        let proc_index_map = AccountProcedureIndexMap::new(foreign_code_commitments, &adv_provider);

        let kernel_assertion_errors = BTreeMap::from(TX_KERNEL_ERRORS);

        Self {
            adv_provider,
            acct_procedure_index_map: proc_index_map.unwrap(),
            mast_store,
            error_messages: kernel_assertion_errors,
        }
    }

    /// Consumes `self` and returns the advice provider and account vault delta.
    pub fn into_parts(self) -> (MemAdviceProvider, AccountVaultDelta) {
        (self.adv_provider, AccountVaultDelta::default())
    }

    // EVENT HANDLERS
    // --------------------------------------------------------------------------------------------

    fn on_push_account_procedure_index(
        &mut self,
        process: ProcessState,
    ) -> Result<(), ExecutionError> {
        let proc_idx = self
            .acct_procedure_index_map
            .get_proc_index(&process)
            .map_err(|err| ExecutionError::EventError(Box::new(err)))?;
        self.adv_provider.push_stack(AdviceSource::Value(proc_idx.into()))?;
        Ok(())
    }
}

impl Host for MockHost {
    type AdviceProvider = MemAdviceProvider;

    fn advice_provider(&self) -> &Self::AdviceProvider {
        &self.adv_provider
    }

    fn advice_provider_mut(&mut self) -> &mut Self::AdviceProvider {
        &mut self.adv_provider
    }

    fn get_mast_forest(&self, node_digest: &Digest) -> Option<Arc<MastForest>> {
        self.mast_store.get(node_digest)
    }

    fn on_event(&mut self, process: ProcessState, event_id: u32) -> Result<(), ExecutionError> {
        let event = TransactionEvent::try_from(event_id)
            .map_err(|err| ExecutionError::EventError(Box::new(err)))?;

        if process.ctx() != ContextId::root() {
            return Err(ExecutionError::EventError(Box::new(
                TransactionEventError::NotRootContext(event_id),
            )));
        }

        match event {
            TransactionEvent::AccountPushProcedureIndex => {
                self.on_push_account_procedure_index(process)
            },
            _ => Ok(()),
        }?;

        Ok(())
    }

    fn on_assert_failed(&mut self, process: ProcessState, err_code: u32) -> ExecutionError {
        let err_msg = self
            .error_messages
            .get(&err_code)
            .map_or("Unknown error".to_string(), |msg| msg.to_string());
        // Add hex representation to message so it can be easily found in MASM code.
        let err_msg = format!("0x{:08X}: {}", err_code, err_msg);
        ExecutionError::FailedAssertion {
            clk: process.clk(),
            err_code,
            err_msg: Some(err_msg),
        }
    }
}
