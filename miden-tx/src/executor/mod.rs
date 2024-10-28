use alloc::sync::Arc;

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{AccountCode, AccountId},
    notes::NoteId,
    transaction::{ExecutedTransaction, TransactionArgs, TransactionInputs},
    vm::StackOutputs,
    MAX_TX_EXECUTION_CYCLES, MIN_TX_EXECUTION_CYCLES, ZERO,
};
use vm_processor::{ExecutionOptions, RecAdviceProvider};
use winter_maybe_async::{maybe_async, maybe_await};

use super::{TransactionExecutorError, TransactionHost};
use crate::auth::TransactionAuthenticator;

mod data_store;
pub use data_store::DataStore;

mod mast_store;
pub use mast_store::TransactionMastStore;

// TRANSACTION EXECUTOR
// ================================================================================================

/// The transaction executor is responsible for executing Miden rollup transactions.
///
/// Transaction execution consists of the following steps:
/// - Fetch the data required to execute a transaction from the [DataStore].
/// - Load the code associated with the transaction into the [TransactionMastStore].
/// - Execute the transaction program and create an [ExecutedTransaction].
///
/// The transaction executor uses dynamic dispatch with trait objects for the [DataStore] and
/// [TransactionAuthenticator], allowing it to be used with different backend implementations.
pub struct TransactionExecutor {
    data_store: Arc<dyn DataStore>,
    mast_store: Arc<TransactionMastStore>,
    authenticator: Option<Arc<dyn TransactionAuthenticator>>,
    exec_options: ExecutionOptions,
}

impl TransactionExecutor {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Creates a new [TransactionExecutor] instance with the specified [DataStore] and
    /// [TransactionAuthenticator].
    pub fn new(
        data_store: Arc<dyn DataStore>,
        authenticator: Option<Arc<dyn TransactionAuthenticator>>,
    ) -> Self {
        const _: () = assert!(MIN_TX_EXECUTION_CYCLES <= MAX_TX_EXECUTION_CYCLES);

        Self {
            data_store,
            mast_store: Arc::new(TransactionMastStore::new()),
            authenticator,
            exec_options: ExecutionOptions::new(
                Some(MAX_TX_EXECUTION_CYCLES),
                MIN_TX_EXECUTION_CYCLES,
                false,
                false,
            )
            .expect("Must not fail while max cycles is more than min trace length"),
        }
    }

    /// Puts the [TransactionExecutor] into debug mode.
    ///
    /// When transaction executor is in debug mode, all transaction-related code (note scripts,
    /// account code) will be compiled and executed in debug mode. This will ensure that all debug
    /// instructions present in the original source code are executed.
    pub fn with_debug_mode(mut self, in_debug_mode: bool) -> Self {
        self.exec_options = ExecutionOptions::new(
            Some(self.exec_options.max_cycles()),
            self.exec_options.expected_cycles(),
            self.exec_options.enable_tracing(),
            in_debug_mode,
        )
        .expect("failed to clone execution options");

        self
    }

    /// Enables tracing for the created instance of [TransactionExecutor].
    ///
    /// When tracing is enabled, the executor will receive tracing events as various stages of the
    /// transaction kernel complete. This enables collecting basic stats about how long different
    /// stages of transaction execution take.
    pub fn with_tracing(mut self) -> Self {
        self.exec_options = self.exec_options.with_tracing();
        self
    }

    // STATE MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Loads the provided code into the internal MAST store and associates the procedures of the
    /// account code with the specified account ID.
    pub fn load_account_code(&mut self, _account_id: AccountId, code: &AccountCode) {
        // TODO: account_id is not used yet, but it could be used to build a procedure map for the
        // loaded accounts
        self.mast_store.load_account_code(code);
    }

    // TRANSACTION EXECUTION
    // --------------------------------------------------------------------------------------------

    /// Prepares and executes a transaction specified by the provided arguments and returns an
    /// [ExecutedTransaction].
    ///
    /// The method first fetches the data required to execute the transaction from the [DataStore]
    /// and compile the transaction into an executable program. Then, it executes the transaction
    /// program and creates an [ExecutedTransaction] object.
    ///
    /// # Errors:
    /// Returns an error if:
    /// - If required data can not be fetched from the [DataStore].
    #[maybe_async]
    pub fn execute_transaction(
        &self,
        account_id: AccountId,
        block_ref: u32,
        notes: &[NoteId],
        tx_args: TransactionArgs,
    ) -> Result<ExecutedTransaction, TransactionExecutorError> {
        let tx_inputs =
            maybe_await!(self.data_store.get_transaction_inputs(account_id, block_ref, notes))
                .map_err(TransactionExecutorError::FetchTransactionInputsFailed)?;

        let (stack_inputs, advice_inputs) =
            TransactionKernel::prepare_inputs(&tx_inputs, &tx_args, None);
        let advice_recorder: RecAdviceProvider = advice_inputs.into();

        // load note script MAST into the MAST store
        self.mast_store.load_transaction_code(&tx_inputs, &tx_args);

        let mut host = TransactionHost::new(
            tx_inputs.account().into(),
            advice_recorder,
            self.mast_store.clone(),
            self.authenticator.clone(),
        )
        .map_err(TransactionExecutorError::TransactionHostCreationFailed)?;

        // execute the transaction kernel
        let result = vm_processor::execute(
            &TransactionKernel::main(),
            stack_inputs,
            &mut host,
            self.exec_options,
        )
        .map_err(TransactionExecutorError::ExecuteTransactionProgramFailed)?;

        build_executed_transaction(tx_args, tx_inputs, result.stack_outputs().clone(), host)
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Creates a new [ExecutedTransaction] from the provided data.
fn build_executed_transaction(
    tx_args: TransactionArgs,
    tx_inputs: TransactionInputs,
    stack_outputs: StackOutputs,
    host: TransactionHost<RecAdviceProvider>,
) -> Result<ExecutedTransaction, TransactionExecutorError> {
    let (advice_recorder, account_delta, output_notes, generated_signatures, tx_progress) =
        host.into_parts();

    let (mut advice_witness, _, map, _store) = advice_recorder.finalize();

    let tx_outputs =
        TransactionKernel::from_transaction_parts(&stack_outputs, &map.into(), output_notes)
            .map_err(TransactionExecutorError::InvalidTransactionOutput)?;

    let final_account = &tx_outputs.account;

    let initial_account = tx_inputs.account();

    if initial_account.id() != final_account.id() {
        return Err(TransactionExecutorError::InconsistentAccountId {
            input_id: initial_account.id(),
            output_id: final_account.id(),
        });
    }

    // make sure nonce delta was computed correctly
    let nonce_delta = final_account.nonce() - initial_account.nonce();
    if nonce_delta == ZERO {
        if account_delta.nonce().is_some() {
            return Err(TransactionExecutorError::InconsistentAccountNonceDelta {
                expected: None,
                actual: account_delta.nonce(),
            });
        }
    } else if final_account.nonce() != account_delta.nonce().unwrap_or_default() {
        return Err(TransactionExecutorError::InconsistentAccountNonceDelta {
            expected: Some(final_account.nonce()),
            actual: account_delta.nonce(),
        });
    }

    // introduce generated signatures into the witness inputs
    advice_witness.extend_map(generated_signatures);

    Ok(ExecutedTransaction::new(
        tx_inputs,
        tx_outputs,
        account_delta,
        tx_args,
        advice_witness,
        tx_progress.into(),
    ))
}
