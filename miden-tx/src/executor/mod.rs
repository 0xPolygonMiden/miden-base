use alloc::{rc::Rc, vec::Vec};

use miden_lib::transaction::{ToTransactionKernelInputs, TransactionKernel};
use miden_objects::{
    notes::NoteScript,
    transaction::{TransactionArgs, TransactionInputs, TransactionScript},
    vm::{Program, StackOutputs},
    Felt, Word, ZERO,
};
use vm_processor::ExecutionOptions;
use winter_maybe_async::{maybe_async, maybe_await};

use super::{
    AccountCode, AccountId, ExecutedTransaction, NoteId, PreparedTransaction, RecAdviceProvider,
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

    /// Fetches the account code from the [DataStore], compiles it, and loads the compiled code
    /// into the internal cache.
    ///
    /// This also returns the [AccountCode] object built from the loaded account code.
    ///
    /// # Errors:
    /// Returns an error if:
    /// - If the account code cannot be fetched from the [DataStore].
    /// - If the account code fails to be loaded into the compiler.
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

    /// Compiles the provided source code into a [NoteScript] and checks (to the extent possible) if
    /// the specified note program could be executed against all accounts with the specified
    /// interfaces.
    pub fn compile_note_script(
        &self,
        note_script: &str,
    ) -> Result<NoteScript, TransactionExecutorError> {
        NoteScript::compile(note_script, TransactionKernel::assembler())
            .map_err(TransactionExecutorError::CompileNoteScriptFailed)
    }

    /// Compiles the provided transaction script source and inputs into a [TransactionScript] and
    /// checks (to the extent possible) that the transaction script can be executed against all
    /// accounts with the specified interfaces.
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
    /// - If the transaction program can not be compiled.
    /// - If the transaction program can not be executed.
    #[maybe_async]
    pub fn execute_transaction(
        &self,
        account_id: AccountId,
        block_ref: u32,
        notes: &[NoteId],
        tx_args: TransactionArgs,
    ) -> Result<ExecutedTransaction, TransactionExecutorError> {
        let transaction =
            maybe_await!(self.prepare_transaction(account_id, block_ref, notes, tx_args))?;

        let (stack_inputs, advice_inputs) = transaction.get_kernel_inputs();
        let advice_recorder: RecAdviceProvider = advice_inputs.into();

        // TODO: load note and tx_scripts into the MAST store
        let mut host = TransactionHost::new(
            transaction.account().into(),
            advice_recorder,
            self.mast_store.clone(),
            self.authenticator.clone(),
        )
        .map_err(TransactionExecutorError::TransactionHostCreationFailed)?;

        let result = vm_processor::execute(
            transaction.program(),
            stack_inputs,
            &mut host,
            self.exec_options,
        )
        .map_err(TransactionExecutorError::ExecuteTransactionProgramFailed)?;

        let (tx_program, tx_inputs, tx_args) = transaction.into_parts();

        build_executed_transaction(
            tx_program,
            tx_args,
            tx_inputs,
            result.stack_outputs().clone(),
            host,
        )
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    /// Fetches the data required to execute the transaction from the [DataStore], compiles the
    /// transaction into an executable program using the [TransactionCompiler], and returns a
    /// [PreparedTransaction].
    ///
    /// # Errors:
    /// Returns an error if:
    /// - If required data can not be fetched from the [DataStore].
    /// - If the transaction can not be compiled.
    #[maybe_async]
    pub fn prepare_transaction(
        &self,
        account_id: AccountId,
        block_ref: u32,
        notes: &[NoteId],
        tx_args: TransactionArgs,
    ) -> Result<PreparedTransaction, TransactionExecutorError> {
        let tx_inputs =
            maybe_await!(self.data_store.get_transaction_inputs(account_id, block_ref, notes))
                .map_err(TransactionExecutorError::FetchTransactionInputsFailed)?;

        let tx_program = TransactionKernel::main().unwrap();

        Ok(PreparedTransaction::new(tx_program, tx_inputs, tx_args))
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Creates a new [ExecutedTransaction] from the provided data.
fn build_executed_transaction<A: TransactionAuthenticator>(
    program: Program,
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
        program,
        tx_inputs,
        tx_outputs,
        account_delta,
        tx_args,
        advice_witness,
    ))
}
