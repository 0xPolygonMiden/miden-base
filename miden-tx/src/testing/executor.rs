use alloc::{rc::Rc, vec::Vec};

use miden_lib::transaction::{ToTransactionKernelInputs, TransactionKernel};
use miden_objects::{
    accounts::{AccountCode, AccountId},
    assembly::ProgramAst,
    notes::{NoteId, NoteScript},
    transaction::{
        ExecutedTransaction, PreparedTransaction, TransactionArgs, TransactionInputs,
        TransactionScript,
    },
    vm::{Program, StackOutputs},
    Felt, Word, ZERO,
};
use vm_processor::{
    AdviceInputs, Digest, ExecutionError, ExecutionOptions, Process, RecAdviceProvider,
};

use super::MockHost;
use crate::{
    auth::TransactionAuthenticator, DataStore, ScriptTarget, TransactionCompiler,
    TransactionExecutorError, TransactionHost,
};

// MOCK TRANSACTION EXECUTOR
// ================================================================================================

pub struct MockExecutor {
    compiler: TransactionCompiler,
    exec_options: ExecutionOptions,
    account_id: AccountId,
    advice_inputs: AdviceInputs,
    host: MockHost,
}

impl MockExecutor {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    pub fn new(mut self) -> Self {
        self.compiler = self.compiler.with_debug_mode(true);
        self.exec_options = self.exec_options.with_debugging();

        self
    }

    pub fn run_tx_with_inputs(
        tx: &PreparedTransaction,
        inputs: AdviceInputs,
    ) -> Result<Process<MockHost>, ExecutionError> {
        let program = tx.program().clone();
        let (stack_inputs, mut advice_inputs) = tx.get_kernel_inputs();
        advice_inputs.extend(inputs);
        let host = MockHost::new(tx.account().into(), advice_inputs);
        let mut process = Process::new_debug(program.kernel().clone(), stack_inputs, host);
        process.execute(&program)?;
        Ok(process)
    }
}

pub struct MockExecutorBuilder {
    compiler: TransactionCompiler,
    exec_options: ExecutionOptions,
    account_id: AccountId,
    advice_inputs: AdviceInputs,
    host: MockHost,
}

impl MockExecutorBuilder {
    pub fn new(account_id: AccountId) -> Self {
        MockExecutorBuilder {
            compiler: Default::default(),
            exec_options: Default::default(),
            account_id,
            advice_inputs: Default::default(),
            host: MockHost::new(account, advice_inputs),
        }
    }

    pub fn advice_inputs(mut self, advice_inputs: AdviceInputs) -> Self {
        self.advice_inputs = advice_inputs;
        self
    }

    pub fn host(mut self, host: MockHost) -> Self {
        self.host = host;
        self
    }

    pub fn build(self) -> MockExecutor {
        MockExecutor {
            compiler: self.compiler,
            exec_options: self.exec_options,
            account_id: self.account_id,
            advice_inputs: self.advice_inputs,
            host: self.host,
        }
    }
}

impl Default for MockExecutorBuilder {
    fn default() -> Self {
        Self::new()
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
