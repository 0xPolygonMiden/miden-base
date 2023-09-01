use super::{
    AccountCode, AccountId, DataStore, Digest, NoteOrigin, NoteScript, NoteTarget,
    PreparedTransaction, ProgramAst, RecAdviceProvider, TransactionComplier,
    TransactionExecutorError, TransactionResult,
};

/// The transaction executor is responsible for executing Miden rollup transactions.
///
/// Transaction execution consists of the following steps:
/// - Fetch the data required to execute a transaction from the [DataStore].
/// - Compile the transaction into a program using the [TransactionComplier].
/// - Execute the transaction program and create a [TransactionWitness].
///
/// The [TransactionExecutor] is generic over the [DataStore] which allows it to be used with
/// different data backend implementations.
///
/// The [TransactionExecutor::execute_transaction()] method is the main entry point for the
/// executor and produces a [TransactionWitness] for the transaction. The TransactionWitness can
/// then be used to by the prover to generate a proof transaction execution.
pub struct TransactionExecutor<D: DataStore> {
    compiler: TransactionComplier,
    data_store: D,
}

impl<D: DataStore> TransactionExecutor<D> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates a new [TransactionExecutor] instance with the specified [DataStore].
    pub fn new(data_store: D) -> Self {
        let compiler = TransactionComplier::new();
        Self {
            compiler,
            data_store,
        }
    }

    // MODIFIERS
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
    pub fn load_account(
        &mut self,
        account_id: AccountId,
    ) -> Result<AccountCode, TransactionExecutorError> {
        let account_code = self
            .data_store
            .get_account_code(account_id)
            .map_err(TransactionExecutorError::FetchAccountCodeFailed)?;
        self.compiler
            .load_account(account_id, account_code)
            .map_err(TransactionExecutorError::LoadAccountFailed)
    }

    /// Loads the provided account interface (vector of procedure digests) into the the compiler.
    ///
    /// Returns the old account interface if it previously existed.
    pub fn load_account_interface(
        &mut self,
        account_id: AccountId,
        procedures: Vec<Digest>,
    ) -> Option<Vec<Digest>> {
        self.compiler.load_account_interface(account_id, procedures)
    }

    /// Compiles the provided program into the [NoteScript] and checks (to the extent possible)
    /// if a note could be executed against all accounts with the specified interfaces.
    pub fn compile_note_script(
        &mut self,
        note_script_ast: ProgramAst,
        target_account_procs: Vec<NoteTarget>,
    ) -> Result<NoteScript, TransactionExecutorError> {
        self.compiler
            .compile_note_script(note_script_ast, target_account_procs)
            .map_err(TransactionExecutorError::CompileNoteScriptFailed)
    }

    /// Prepares and executes a transaction specified by the provided arguments and returns a
    /// [TransactionWitness].
    ///
    /// The method first fetches the data required to execute the transaction from the [DataStore]
    /// and compile the transaction into an executable program. Then it executes the transaction
    /// program and execute_transactioncreates a [TransactionWitness].
    ///
    /// # Errors:
    /// Returns an error if:
    /// - If required data can not be fetched from the [DataStore].
    /// - If the transaction program can not be compiled.
    /// - If the transaction program can not be executed.
    pub fn (
        &mut self,
        account_id: AccountId,
        block_ref: u32,
        note_origins: &[NoteOrigin],
        tx_script: Option<ProgramAst>,
    ) -> Result<TransactionResult, TransactionExecutorError> {
        let transaction =
            self.prepare_transaction(account_id, block_ref, note_origins, tx_script)?;

        let mut advice_recorder: RecAdviceProvider = transaction.advice_provider_inputs().into();
        let result = processor::execute(
            transaction.tx_program(),
            transaction.stack_inputs(),
            &mut advice_recorder,
            Default::default(),
        )
        .map_err(TransactionExecutorError::ExecuteTransactionProgramFailed)?;

        let (account, block_header, _block_chain, consumed_notes, tx_program, tx_script_root) =
            transaction.into_parts();

        TransactionResult::new(
            account,
            consumed_notes,
            block_header.hash(),
            tx_program,
            tx_script_root,
            advice_recorder,
            result.stack_outputs().clone(),
        )
        .map_err(TransactionExecutorError::TransactionResultError)
    }

    /// Fetches the data required to execute the transaction from the [DataStore], compiles the
    /// transaction into an executable program using the [TransactionComplier], and returns a
    /// [PreparedTransaction].
    ///
    /// # Errors:
    /// Returns an error if:
    /// - If required data can not be fetched from the [DataStore].
    /// - If the transaction can not be compiled.
    pub fn prepare_transaction(
        &mut self,
        account_id: AccountId,
        block_ref: u32,
        note_origins: &[NoteOrigin],
        tx_script: Option<ProgramAst>,
    ) -> Result<PreparedTransaction, TransactionExecutorError> {
        let (account, block_header, block_chain, notes) = self
            .data_store
            .get_transaction_data(account_id, block_ref, note_origins)
            .map_err(TransactionExecutorError::FetchTransactionDataFailed)?;

        let (tx_program, tx_script_root) = self
            .compiler
            .compile_transaction(account_id, &notes, tx_script)
            .map_err(TransactionExecutorError::CompileTransactionError)?;

        PreparedTransaction::new(
            account,
            None,
            block_header,
            block_chain,
            notes,
            tx_script_root,
            tx_program,
        )
        .map_err(TransactionExecutorError::ConstructPreparedTransactionFailed)
    }
}
