use alloc::{collections::BTreeSet, sync::Arc, vec::Vec};

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    Felt, MAX_TX_EXECUTION_CYCLES, MIN_TX_EXECUTION_CYCLES, ZERO,
    account::AccountId,
    block::BlockNumber,
    transaction::{
        ExecutedTransaction, ForeignAccountInputs, InputNote, InputNotes, TransactionArgs,
        TransactionInputs, TransactionScript,
    },
    vm::StackOutputs,
};
pub use vm_processor::MastForestStore;
use vm_processor::{AdviceInputs, ExecutionOptions, Process, RecAdviceProvider};
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
/// - Execute the transaction program and create an [ExecutedTransaction].
///
/// The transaction executor uses dynamic dispatch with trait objects for the [DataStore] and
/// [TransactionAuthenticator], allowing it to be used with different backend implementations.
/// At the moment of execution, the [DataStore] is expected to provide all required MAST nodes.
pub struct TransactionExecutor {
    data_store: Arc<dyn DataStore>,
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
    pub fn with_debug_mode(mut self) -> Self {
        self.exec_options = self.exec_options.with_debugging();
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
    /// - If the transaction arguments contain foreign account data not anchored in the reference
    ///   block.
    /// - If any input notes were created in block numbers higher than the reference block.
    #[maybe_async]
    pub fn execute_transaction(
        &self,
        account_id: AccountId,
        block_ref: BlockNumber,
        notes: InputNotes<InputNote>,
        tx_args: TransactionArgs,
    ) -> Result<ExecutedTransaction, TransactionExecutorError> {
        // Validate that notes were not created after the reference, and build the set of required
        // block numbers
        let mut ref_blocks: BTreeSet<BlockNumber> = BTreeSet::new();
        for note in &notes {
            if let Some(location) = note.location() {
                if location.block_num() > block_ref {
                    return Err(TransactionExecutorError::NoteBlockPastReferenceBlock(
                        note.id(),
                        block_ref,
                    ));
                }
                ref_blocks.insert(location.block_num());
            }
        }

        ref_blocks.insert(block_ref);

        let (account, seed, ref_header, mmr) =
            maybe_await!(self.data_store.get_transaction_inputs(account_id, ref_blocks))
                .map_err(TransactionExecutorError::FetchTransactionInputsFailed)?;

        // Validate that foreign account inputs are anchored in the reference block
        for foreign_account in tx_args.foreign_accounts() {
            let computed_account_root = foreign_account.compute_account_root().map_err(|err| {
                TransactionExecutorError::InvalidAccountWitness(foreign_account.id(), err)
            })?;
            if computed_account_root != ref_header.account_root() {
                return Err(TransactionExecutorError::ForeignAccountNotAnchoredInReference(
                    foreign_account.id(),
                ));
            }
        }

        let tx_inputs = TransactionInputs::new(account, seed, ref_header, mmr, notes)
            .map_err(TransactionExecutorError::InvalidTransactionInputs)?;

        let (stack_inputs, advice_inputs) =
            TransactionKernel::prepare_inputs(&tx_inputs, &tx_args, None)
                .map_err(TransactionExecutorError::InvalidTransactionInputs)?;

        let advice_recorder: RecAdviceProvider = advice_inputs.into();

        let mut host = TransactionHost::new(
            tx_inputs.account().into(),
            advice_recorder,
            self.data_store.clone(),
            self.authenticator.clone(),
            tx_args.foreign_account_code_commitments(),
        )
        .map_err(TransactionExecutorError::TransactionHostCreationFailed)?;

        // Execute the transaction kernel
        let result = vm_processor::execute(
            &TransactionKernel::main(),
            stack_inputs,
            &mut host,
            self.exec_options,
        )
        .map_err(TransactionExecutorError::TransactionProgramExecutionFailed)?;

        build_executed_transaction(tx_args, tx_inputs, result.stack_outputs().clone(), host)
    }

    // SCRIPT EXECUTION
    // --------------------------------------------------------------------------------------------

    /// Executes an arbitrary script against the given account and returns the stack state at the
    /// end of execution.
    ///
    /// # Errors:
    /// Returns an error if:
    /// - If required data can not be fetched from the [DataStore].
    /// - If the transaction host can not be created from the provided values.
    /// - If the execution of the provided program fails.
    #[maybe_async]
    pub fn execute_tx_view_script(
        &self,
        account_id: AccountId,
        block_ref: BlockNumber,
        tx_script: TransactionScript,
        advice_inputs: AdviceInputs,
        foreign_account_inputs: Vec<ForeignAccountInputs>,
    ) -> Result<[Felt; 16], TransactionExecutorError> {
        let ref_blocks = [block_ref].into_iter().collect();
        let (account, seed, header, mmr) =
            maybe_await!(self.data_store.get_transaction_inputs(account_id, ref_blocks))
                .map_err(TransactionExecutorError::FetchTransactionInputsFailed)?;

        let tx_inputs = TransactionInputs::new(account, seed, header, mmr, Default::default())
            .map_err(TransactionExecutorError::InvalidTransactionInputs)?;

        let tx_args = TransactionArgs::new(
            Some(tx_script.clone()),
            None,
            Default::default(),
            foreign_account_inputs,
        );

        let (stack_inputs, advice_inputs) =
            TransactionKernel::prepare_inputs(&tx_inputs, &tx_args, Some(advice_inputs))
                .map_err(TransactionExecutorError::InvalidTransactionInputs)?;
        let advice_recorder: RecAdviceProvider = advice_inputs.into();

        let mut host = TransactionHost::new(
            tx_inputs.account().into(),
            advice_recorder,
            self.data_store.clone(),
            self.authenticator.clone(),
            tx_args.foreign_account_code_commitments(),
        )
        .map_err(TransactionExecutorError::TransactionHostCreationFailed)?;

        let mut process = Process::new(
            TransactionKernel::tx_script_main().kernel().clone(),
            stack_inputs,
            self.exec_options,
        );
        let stack_outputs = process
            .execute(&TransactionKernel::tx_script_main(), &mut host)
            .map_err(TransactionExecutorError::TransactionProgramExecutionFailed)?;

        Ok(*stack_outputs)
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
            .map_err(TransactionExecutorError::TransactionOutputConstructionFailed)?;

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
