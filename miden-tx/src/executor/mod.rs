use super::{
    AccountCode, AccountId, DataStore, Digest, NoteOrigin, NoteScript, NoteTarget,
    PreparedTransaction, RecAdviceProvider, TransactionCompiler, TransactionExecutorError,
    TransactionResult,
};
use crate::TryFromVmResult;
use miden_lib::transaction::{extract_account_storage_delta, extract_account_vault_delta};
use miden_objects::{
    accounts::{Account, AccountDelta},
    assembly::ProgramAst,
    transaction::{ConsumedNotes, CreatedNotes, FinalAccountStub},
    TransactionResultError,
};
use vm_core::{Program, StackOutputs};

use super::TransactionHost;

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
    compiler: TransactionCompiler,
    data_store: D,
}

impl<D: DataStore> TransactionExecutor<D> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates a new [TransactionExecutor] instance with the specified [DataStore].
    pub fn new(data_store: D) -> Self {
        let compiler = TransactionCompiler::new();
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
    /// program and creates a [TransactionWitness].
    ///
    /// # Errors:
    /// Returns an error if:
    /// - If required data can not be fetched from the [DataStore].
    /// - If the transaction program can not be compiled.
    /// - If the transaction program can not be executed.
    pub fn execute_transaction(
        &mut self,
        account_id: AccountId,
        block_ref: u32,
        note_origins: &[NoteOrigin],
        tx_script: Option<ProgramAst>,
    ) -> Result<TransactionResult, TransactionExecutorError> {
        let transaction =
            self.prepare_transaction(account_id, block_ref, note_origins, tx_script)?;

        let advice_recorder: RecAdviceProvider = transaction.advice_provider_inputs().into();
        let mut host = TransactionHost::new(advice_recorder);
        let result = vm_processor::execute(
            transaction.tx_program(),
            transaction.stack_inputs(),
            &mut host,
            Default::default(),
        )
        .map_err(TransactionExecutorError::ExecuteTransactionProgramFailed)?;

        let (account, block_header, _block_chain, consumed_notes, tx_program, tx_script_root) =
            transaction.into_parts();

        let (advice_recorder, event_handler) = host.into_parts();
        println!("event_handler: {:?}", event_handler);
        create_transaction_result(
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

/// Creates a new [TransactionResult] from the provided data, advice provider and stack outputs.
pub fn create_transaction_result(
    initial_account: Account,
    consumed_notes: ConsumedNotes,
    block_hash: Digest,
    program: Program,
    tx_script_root: Option<Digest>,
    advice_provider: RecAdviceProvider,
    stack_outputs: StackOutputs,
) -> Result<TransactionResult, TransactionResultError> {
    // finalize the advice recorder
    let (advice_witness, stack, map, store) = advice_provider.finalize();

    // parse transaction results
    let final_account_stub =
        FinalAccountStub::try_from_vm_result(&stack_outputs, &stack, &map, &store)?;
    let created_notes = CreatedNotes::try_from_vm_result(&stack_outputs, &stack, &map, &store)?;

    // TODO: Fix delta extraction for new account creation
    // extract the account storage delta
    let storage_delta =
        extract_account_storage_delta(&store, &initial_account, &final_account_stub)?;

    // extract the nonce delta
    let nonce_delta = if initial_account.nonce() != final_account_stub.0.nonce() {
        Some(final_account_stub.0.nonce())
    } else {
        None
    };

    // extract vault delta
    let vault_delta =
        extract_account_vault_delta(&store, &map, &initial_account, &final_account_stub)?;

    // construct the account delta
    let account_delta = AccountDelta {
        code: None,
        nonce: nonce_delta,
        storage: storage_delta,
        vault: vault_delta,
    };

    TransactionResult::new(
        initial_account,
        final_account_stub,
        account_delta,
        consumed_notes,
        created_notes,
        block_hash,
        program,
        tx_script_root,
        advice_witness,
    )
}
