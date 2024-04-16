use alloc::vec::Vec;

use miden_lib::transaction::{ToTransactionKernelInputs, TransactionKernel};
use miden_objects::{
    assembly::ProgramAst,
    transaction::{TransactionArgs, TransactionInputs, TransactionScript},
    vm::{Program, StackOutputs},
    Felt, Word, ZERO,
};
use vm_processor::ExecutionOptions;

use super::{
    AccountCode, AccountId, Digest, ExecutedTransaction, NoteId, NoteScript, PreparedTransaction,
    RecAdviceProvider, ScriptTarget, TransactionCompiler, TransactionExecutorError,
    TransactionHost,
};

mod data;
pub use data::DataStore;

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
pub struct TransactionExecutor {
    compiler: TransactionCompiler,
    exec_options: ExecutionOptions,
}

impl Default for TransactionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionExecutor {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates a new [TransactionExecutor] instance with the specified [DataStore].
    pub fn new() -> Self {
        Self {
            compiler: TransactionCompiler::new(),
            exec_options: ExecutionOptions::default(),
        }
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
    pub fn load_account<D: DataStore>(
        &mut self,
        account_id: AccountId,
        data_store: &D,
    ) -> Result<AccountCode, TransactionExecutorError> {
        let account_code = data_store
            .get_account_code(account_id)
            .map_err(TransactionExecutorError::FetchAccountCodeFailed)?;
        self.compiler
            .load_account(account_id, account_code)
            .map_err(TransactionExecutorError::LoadAccountFailed)
    }

    /// Loads the provided account interface (vector of procedure digests) into the compiler.
    ///
    /// Returns the old interface for the specified account ID if it previously existed.
    pub fn load_account_interface(
        &mut self,
        account_id: AccountId,
        procedures: Vec<Digest>,
    ) -> Option<Vec<Digest>> {
        self.compiler.load_account_interface(account_id, procedures)
    }

    // COMPILERS
    // --------------------------------------------------------------------------------------------

    /// Compiles the provided program into a [NoteScript] and checks (to the extent possible) if
    /// the specified note program could be executed against all accounts with the specified
    /// interfaces.
    pub fn compile_note_script(
        &self,
        note_script_ast: ProgramAst,
        target_account_procs: Vec<ScriptTarget>,
    ) -> Result<NoteScript, TransactionExecutorError> {
        self.compiler
            .compile_note_script(note_script_ast, target_account_procs)
            .map_err(TransactionExecutorError::CompileNoteScriptFailed)
    }

    /// Compiles the provided transaction script source and inputs into a [TransactionScript] and
    /// checks (to the extent possible) that the transaction script can be executed against all
    /// accounts with the specified interfaces.
    pub fn compile_tx_script<T>(
        &self,
        tx_script_ast: ProgramAst,
        inputs: T,
        target_account_procs: Vec<ScriptTarget>,
    ) -> Result<TransactionScript, TransactionExecutorError>
    where
        T: IntoIterator<Item = (Word, Vec<Felt>)>,
    {
        self.compiler
            .compile_tx_script(tx_script_ast, inputs, target_account_procs)
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
    pub fn execute_transaction<D: DataStore>(
        &self,
        account_id: AccountId,
        block_ref: u32,
        notes: &[NoteId],
        tx_args: TransactionArgs,
        data_store: &D,
    ) -> Result<ExecutedTransaction, TransactionExecutorError> {
        let transaction =
            self.prepare_transaction(account_id, block_ref, notes, tx_args, data_store)?;

        let (stack_inputs, advice_inputs) = transaction.get_kernel_inputs();
        let advice_recorder: RecAdviceProvider = advice_inputs.into();
        let mut host = TransactionHost::new(transaction.account().into(), advice_recorder);

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
    fn prepare_transaction<D: DataStore>(
        &self,
        account_id: AccountId,
        block_ref: u32,
        notes: &[NoteId],
        tx_args: TransactionArgs,
        data_store: &D,
    ) -> Result<PreparedTransaction, TransactionExecutorError> {
        let tx_inputs = data_store
            .get_transaction_inputs(account_id, block_ref, notes)
            .map_err(TransactionExecutorError::FetchTransactionInputsFailed)?;

        let tx_program = self
            .compiler
            .compile_transaction(
                account_id,
                tx_inputs.input_notes(),
                tx_args.tx_script().map(|x| x.code()),
            )
            .map_err(TransactionExecutorError::CompileTransactionFailed)?;

        Ok(PreparedTransaction::new(tx_program, tx_inputs, tx_args))
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Creates a new [ExecutedTransaction] from the provided data.
fn build_executed_transaction(
    program: Program,
    tx_args: TransactionArgs,
    tx_inputs: TransactionInputs,
    stack_outputs: StackOutputs,
    host: TransactionHost<RecAdviceProvider>,
) -> Result<ExecutedTransaction, TransactionExecutorError> {
    let (advice_recorder, account_delta, output_notes) = host.into_parts();

    let (advice_witness, _, map, _store) = advice_recorder.finalize();

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

    Ok(ExecutedTransaction::new(
        program,
        tx_inputs,
        tx_outputs,
        account_delta,
        tx_args,
        advice_witness,
    ))
}
