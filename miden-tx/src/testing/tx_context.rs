use alloc::{rc::Rc, vec::Vec};
use std::{env, path::PathBuf, println, sync::Arc};

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountId},
    assembly::{Assembler, Library, LibraryNamespace},
    notes::{Note, NoteId},
    transaction::{ExecutedTransaction, InputNote, InputNotes, TransactionArgs, TransactionInputs},
};
use vm_processor::{AdviceInputs, ExecutionError, Felt, Process};
use winter_maybe_async::{maybe_async, maybe_await};

use super::{
    executor::CodeExecutor,
    mock_chain::{MockAuthenticator, MockChain},
    MockHost,
};
use crate::{
    DataStore, DataStoreError, TransactionExecutor, TransactionExecutorError, TransactionMastStore,
};

mod builder;
pub use builder::TransactionContextBuilder;

// TRANSACTION CONTEXT
// ================================================================================================

#[derive(Clone)]
/// Represents all needed data for executing a transaction, or arbitrary code.
///
/// It implements [DataStore], so transactions may be executed with [TransactionExecutor](crate::TransactionExecutor)
pub struct TransactionContext {
    mock_chain: MockChain,
    expected_output_notes: Vec<Note>,
    tx_args: TransactionArgs,
    tx_inputs: TransactionInputs,
    advice_inputs: AdviceInputs,
    authenticator: Option<MockAuthenticator>,
    assembler: Assembler,
}
impl TransactionContext {
    /// Executes arbitrary code
    pub fn execute_code(&self, code: &str) -> Result<Process<MockHost>, ExecutionError> {
        let (stack_inputs, advice_inputs) = TransactionKernel::prepare_inputs(
            &self.tx_inputs,
            &self.tx_args,
            Some(self.advice_inputs.clone()),
        );

        let workspace_dir = env!("CARGO_MANIFEST_DIR");
        let path = PathBuf::from(format!("{workspace_dir}/../miden-lib/asm/kernels/transaction/"));
        let namespace = "kernel".parse::<LibraryNamespace>().expect("invalid base namespace");
        let test_lib =
            Library::from_dir(path.join("lib"), namespace, TransactionKernel::assembler()).unwrap();

        let mast_store = Rc::new(TransactionMastStore::new());
        mast_store.insert(Arc::new(test_lib.mast_forest().clone()));
        let program = self.assembler.clone().assemble_program(code).unwrap();
        mast_store.insert(Arc::new(program.mast_forest().clone()));
        mast_store.load_transaction_code(&self.tx_inputs, &self.tx_args);

        CodeExecutor::new(MockHost::new(self.tx_inputs.account().into(), advice_inputs, mast_store))
            .stack_inputs(stack_inputs)
            .execute_program(program)
    }

    /// Executes the transaction through a [TransactionExecutor]
    #[maybe_async]
    pub fn execute(self) -> Result<ExecutedTransaction, TransactionExecutorError> {
        let mock_data_store = MockDataStore::new(self.tx_inputs.clone());

        let account_id = self.account().id();
        let block_num = mock_data_store.tx_inputs.block_header().block_num();
        let tx_executor =
            TransactionExecutor::new(mock_data_store, self.authenticator.map(Rc::new));
        let notes: Vec<NoteId> = self.tx_inputs.input_notes().into_iter().map(|n| n.id()).collect();

        maybe_await!(tx_executor.execute_transaction(account_id, block_num, &notes, self.tx_args))
    }

    pub fn account(&self) -> &Account {
        self.tx_inputs.account()
    }

    pub fn expected_output_notes(&self) -> &[Note] {
        &self.expected_output_notes
    }

    pub fn mock_chain(&self) -> &MockChain {
        &self.mock_chain
    }

    pub fn input_notes(&self) -> InputNotes<InputNote> {
        InputNotes::new(self.mock_chain.available_notes().clone()).unwrap()
    }

    pub fn tx_args(&self) -> &TransactionArgs {
        &self.tx_args
    }

    pub fn set_tx_args(&mut self, tx_args: TransactionArgs) {
        self.tx_args = tx_args;
    }

    pub fn tx_inputs(&self) -> &TransactionInputs {
        &self.tx_inputs
    }
}

impl DataStore for TransactionContext {
    #[maybe_async]
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteId],
    ) -> Result<TransactionInputs, DataStoreError> {
        assert_eq!(account_id, self.tx_inputs.account().id());
        assert_eq!(block_num, self.tx_inputs.block_header().block_num());
        assert_eq!(notes.len(), self.tx_inputs.input_notes().num_notes());

        Ok(self.tx_inputs.clone())
    }
}

struct MockDataStore {
    tx_inputs: TransactionInputs,
}

impl MockDataStore {
    fn new(tx_inputs: TransactionInputs) -> Self {
        MockDataStore { tx_inputs }
    }
}

impl DataStore for MockDataStore {
    #[maybe_async]
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteId],
    ) -> Result<TransactionInputs, DataStoreError> {
        assert_eq!(account_id, self.tx_inputs.account().id());
        assert_eq!(block_num, self.tx_inputs.block_header().block_num());
        assert_eq!(notes.len(), self.tx_inputs.input_notes().num_notes());

        Ok(self.tx_inputs.clone())
    }
}
