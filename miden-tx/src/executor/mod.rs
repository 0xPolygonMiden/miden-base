use miden_lib::{outputs::TX_SCRIPT_ROOT_WORD_IDX, transaction::extract_account_storage_delta};
use miden_objects::{
    accounts::{Account, AccountDelta},
    assembly::ProgramAst,
    transaction::{FinalAccountStub, InputNotes, OutputNotes, TransactionScript},
    Felt, TransactionResultError, Word, WORD_SIZE,
};
use vm_core::{Program, StackOutputs, StarkField};

use super::{
    AccountCode, AccountId, DataStore, Digest, NoteOrigin, NoteScript, PreparedTransaction,
    RecAdviceProvider, ScriptTarget, TransactionCompiler, TransactionExecutorError,
    TransactionHost, TransactionResult,
};
use crate::{host::EventHandler, TryFromVmResult};

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
        Self { compiler, data_store }
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
        &mut self,
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
        tx_script: Option<TransactionScript>,
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

        let (account, block_header, _block_chain, consumed_notes, tx_program, tx_script) =
            transaction.into_parts();

        let (advice_recorder, event_handler) = host.into_parts();
        create_transaction_result(
            account,
            consumed_notes,
            block_header.hash(),
            tx_program,
            tx_script.map(|s| *s.hash()),
            advice_recorder,
            result.stack_outputs().clone(),
            event_handler,
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
        tx_script: Option<TransactionScript>,
    ) -> Result<PreparedTransaction, TransactionExecutorError> {
        let tx_inputs = self
            .data_store
            .get_transaction_inputs(account_id, block_ref, note_origins)
            .map_err(TransactionExecutorError::FetchTransactionInputsFailed)?;

        let tx_program = self
            .compiler
            .compile_transaction(
                account_id,
                &tx_inputs.input_notes,
                tx_script.as_ref().map(|x| x.code()),
            )
            .map_err(TransactionExecutorError::CompileTransactionError)?;

        PreparedTransaction::new(tx_program, tx_script, tx_inputs)
            .map_err(TransactionExecutorError::ConstructPreparedTransactionFailed)
    }
}

#[allow(clippy::too_many_arguments)]
/// Creates a new [TransactionResult] from the provided data, advice provider and stack outputs.
pub fn create_transaction_result(
    initial_account: Account,
    input_notes: InputNotes,
    block_hash: Digest,
    program: Program,
    tx_script_root: Option<Digest>,
    advice_provider: RecAdviceProvider,
    stack_outputs: StackOutputs,
    event_handler: EventHandler,
) -> Result<TransactionResult, TransactionResultError> {
    // finalize the advice recorder
    let (advice_witness, stack, map, store) = advice_provider.finalize();

    // parse transaction results
    let final_account_stub =
        FinalAccountStub::try_from_vm_result(&stack_outputs, &stack, &map, &store)?;
    let output_notes = OutputNotes::try_from_vm_result(&stack_outputs, &stack, &map, &store)?;

    // assert the tx_script_root is consistent with the output stack
    debug_assert_eq!(
        (*tx_script_root.unwrap_or_default())
            .into_iter()
            .rev()
            .map(|x| x.as_int())
            .collect::<Vec<_>>(),
        stack_outputs.stack()
            [TX_SCRIPT_ROOT_WORD_IDX * WORD_SIZE..(TX_SCRIPT_ROOT_WORD_IDX + 1) * WORD_SIZE]
    );

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

    // finalize the event handler
    let vault_delta = event_handler.finalize();

    // construct the account delta
    let account_delta =
        AccountDelta::new(storage_delta, vault_delta, nonce_delta).expect("invalid account delta");

    TransactionResult::new(
        initial_account,
        final_account_stub,
        account_delta,
        input_notes,
        output_notes,
        block_hash,
        program,
        tx_script_root,
        advice_witness,
    )
}
