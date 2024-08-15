use alloc::{rc::Rc, vec::Vec};

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    notes::NoteScript,
    transaction::{TransactionArgs, TransactionInputs, TransactionScript},
    vm::StackOutputs,
    Felt, Word, ZERO,
};
use vm_processor::ExecutionOptions;
use winter_maybe_async::{maybe_async, maybe_await};

use super::{
    AccountCode, AccountId, ExecutedTransaction, NoteId, RecAdviceProvider,
    TransactionExecutorError, TransactionHost,
};
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
/// - Compile the transaction into a program using the [TransactionCompiler](crate::TransactionCompiler).
/// - Execute the transaction program and create an [ExecutedTransaction].
///
/// The transaction executor is generic over the [DataStore] which allows it to be used with
/// different data backend implementations.
///
/// The [TransactionExecutor::execute_transaction()] method is the main entry point for the
/// executor and produces an [ExecutedTransaction] for the transaction. The executed transaction
/// can then be used to by the prover to generate a proof transaction execution.
pub struct TransactionExecutor<D, A> {
    data_store: D,
    mast_store: Rc<TransactionMastStore>,
    authenticator: Option<Rc<A>>,
    exec_options: ExecutionOptions,
}

impl<D: DataStore, A: TransactionAuthenticator> TransactionExecutor<D, A> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Creates a new [TransactionExecutor] instance with the specified [DataStore] and [TransactionAuthenticator].
    pub fn new(data_store: D, authenticator: Option<Rc<A>>) -> Self {
        Self {
            data_store,
            mast_store: Rc::new(TransactionMastStore::new()),
            authenticator,
            exec_options: ExecutionOptions::default(),
        }
    }

    /// Puts the [TransactionExecutor] into debug mode.
    ///
    /// When transaction executor is in debug mode, all transaction-related code (note scripts,
    /// account code) will be compiled and executed in debug mode. This will ensure that all debug
    /// instructions present in the original source code are executed.
    pub fn with_debug_mode(mut self, in_debug_mode: bool) -> Self {
        if in_debug_mode && !self.exec_options.enable_debugging() {
            self.exec_options = self.exec_options.with_debugging();
        } else if !in_debug_mode && self.exec_options.enable_debugging() {
            // since we can't set the debug mode directly, we re-create execution options using
            // the same values as current execution options (except for debug mode which defaults
            // to false)
            self.exec_options = ExecutionOptions::new(
                Some(self.exec_options.max_cycles()),
                self.exec_options.expected_cycles(),
                self.exec_options.enable_tracing(),
            )
            .expect("failed to clone execution options");
        }

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

    /// Fetches the account code from the [DataStore], and loads the into the internal cache.
    ///
    /// TODO: remove this as we can load the code on execute_transaction() call?
    ///
    /// # Errors:
    /// Returns an error if the account code cannot be fetched from the [DataStore].
    #[maybe_async]
    pub fn load_account(
        &mut self,
        account_id: AccountId,
    ) -> Result<AccountCode, TransactionExecutorError> {
        let account_code = maybe_await!(self.data_store.get_account_code(account_id))
            .map_err(TransactionExecutorError::FetchAccountCodeFailed)?;
        self.mast_store.load_account(account_code.clone());
        Ok(account_code)
    }

    // COMPILERS
    // --------------------------------------------------------------------------------------------

    /// TODO: remove as this is just a wrapper around [NoteScript::compile()].
    pub fn compile_note_script(
        &self,
        note_script: &str,
    ) -> Result<NoteScript, TransactionExecutorError> {
        NoteScript::compile(note_script, TransactionKernel::assembler())
            .map_err(TransactionExecutorError::CompileNoteScriptFailed)
    }

    /// TODO: remove as this is just a wrapper around [TransactionScript::compile()].
    pub fn compile_tx_script(
        &self,
        tx_script: &str,
        inputs: impl IntoIterator<Item = (Word, Vec<Felt>)>,
    ) -> Result<TransactionScript, TransactionExecutorError> {
        TransactionScript::compile(tx_script, inputs, TransactionKernel::assembler())
            .map_err(TransactionExecutorError::CompileTransactionScriptFailed)
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

        let (stack_inputs, advice_inputs) = TransactionKernel::prepare_inputs(&tx_inputs, &tx_args);
        let advice_recorder: RecAdviceProvider = advice_inputs.into();

        // load note script MAST into the MAST store
        for note in tx_inputs.input_notes() {
            self.mast_store.load_note_script(note.note().script())
        }

        // load tx script MAST into the MAST store
        if let Some(tx_script) = tx_args.tx_script() {
            self.mast_store.load_tx_script(tx_script);
        }

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
fn build_executed_transaction<A: TransactionAuthenticator>(
    tx_args: TransactionArgs,
    tx_inputs: TransactionInputs,
    stack_outputs: StackOutputs,
    host: TransactionHost<RecAdviceProvider, A>,
) -> Result<ExecutedTransaction, TransactionExecutorError> {
    let (advice_recorder, account_delta, output_notes, generated_signatures) = host.into_parts();

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

    // introduce generated signature into the witness inputs
    advice_witness.extend_map(generated_signatures);

    Ok(ExecutedTransaction::new(
        tx_inputs,
        tx_outputs,
        account_delta,
        tx_args,
        advice_witness,
    ))
}
