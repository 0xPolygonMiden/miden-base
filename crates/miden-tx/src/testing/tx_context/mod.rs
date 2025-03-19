#[cfg(feature = "async")]
use alloc::boxed::Box;
use alloc::{rc::Rc, sync::Arc, vec::Vec};

use builder::MockAuthenticator;
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    account::{Account, AccountCode, AccountId},
    assembly::Assembler,
    block::BlockNumber,
    note::{Note, NoteId},
    transaction::{ExecutedTransaction, InputNote, InputNotes, TransactionArgs, TransactionInputs},
};
use vm_processor::{AdviceInputs, ExecutionError, Process};
use winter_maybe_async::*;

use super::{MockHost, executor::CodeExecutor};
use crate::{
    DataStore, DataStoreError, TransactionExecutor, TransactionExecutorError, TransactionMastStore,
    auth::TransactionAuthenticator,
};

mod builder;
pub use builder::TransactionContextBuilder;

// TRANSACTION CONTEXT
// ================================================================================================

#[derive(Clone)]
/// Represents all needed data for executing a transaction, or arbitrary code.
///
/// It implements [DataStore], so transactions may be executed with
/// [TransactionExecutor](crate::TransactionExecutor)
pub struct TransactionContext {
    expected_output_notes: Vec<Note>,
    tx_args: TransactionArgs,
    tx_inputs: TransactionInputs,
    foreign_codes: Vec<AccountCode>,
    advice_inputs: AdviceInputs,
    authenticator: Option<MockAuthenticator>,
    assembler: Assembler,
}

impl TransactionContext {
    /// Executes arbitrary code within the context of a mocked transaction environment and returns
    /// the resulting [Process].
    ///
    /// The code is compiled with the assembler attached to this context and executed with advice
    /// inputs constructed from the data stored in the context. The program is run on a [MockHost]
    /// which is loaded with the procedures exposed by the transaction kernel, and also individual
    /// kernel functions (not normally exposed).
    ///
    /// # Errors
    /// Returns an error if the assembly or execution of the provided code fails.
    pub fn execute_code(&self, code: &str) -> Result<Process, ExecutionError> {
        let (stack_inputs, mut advice_inputs) = TransactionKernel::prepare_inputs(
            &self.tx_inputs,
            &self.tx_args,
            Some(self.advice_inputs.clone()),
        );
        advice_inputs.extend(self.advice_inputs.clone());

        let mast_store = Rc::new(TransactionMastStore::new());

        let test_lib = TransactionKernel::kernel_as_library();
        mast_store.insert(test_lib.mast_forest().clone());

        let program = self
            .assembler
            .clone()
            .with_debug_mode(true)
            .assemble_program(code)
            .expect("compilation of the provided code failed");
        mast_store.insert(program.mast_forest().clone());

        for code in &self.foreign_codes {
            mast_store.insert(code.mast());
        }

        mast_store.load_transaction_code(&self.tx_inputs, &self.tx_args);

        CodeExecutor::new(MockHost::new(
            self.tx_inputs.account().into(),
            advice_inputs,
            mast_store,
            self.foreign_codes.iter().map(|code| code.commitment()).collect(),
        ))
        .stack_inputs(stack_inputs)
        .execute_program(program)
    }

    /// Executes the transaction through a [TransactionExecutor]
    #[maybe_async]
    pub fn execute(self) -> Result<ExecutedTransaction, TransactionExecutorError> {
        let account_id = self.account().id();
        let block_num = self.tx_inputs().block_header().block_num();
        let notes: Vec<NoteId> =
            self.tx_inputs().input_notes().into_iter().map(|n| n.id()).collect();

        let authenticator = self
            .authenticator
            .map(|auth| Arc::new(auth) as Arc<dyn TransactionAuthenticator>);

        let mut tx_executor = TransactionExecutor::new(Arc::new(self.tx_inputs), authenticator);

        for code in self.foreign_codes {
            tx_executor.load_account_code(&code);
        }

        maybe_await!(tx_executor.execute_transaction(account_id, block_num, &notes, self.tx_args))
    }

    pub fn account(&self) -> &Account {
        self.tx_inputs.account()
    }

    pub fn expected_output_notes(&self) -> &[Note] {
        &self.expected_output_notes
    }

    pub fn tx_args(&self) -> &TransactionArgs {
        &self.tx_args
    }

    pub fn input_notes(&self) -> &InputNotes<InputNote> {
        self.tx_inputs.input_notes()
    }

    pub fn set_tx_args(&mut self, tx_args: TransactionArgs) {
        self.tx_args = tx_args;
    }

    pub fn tx_inputs(&self) -> &TransactionInputs {
        &self.tx_inputs
    }

    pub fn get_data_store(&self) -> Arc<dyn DataStore> {
        Arc::new(self.tx_inputs().clone())
    }
}

#[maybe_async_trait]
impl DataStore for TransactionInputs {
    #[maybe_async]
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        block_num: BlockNumber,
        notes: &[NoteId],
    ) -> Result<TransactionInputs, DataStoreError> {
        assert_eq!(account_id, self.account().id());
        assert_eq!(block_num, self.block_header().block_num());
        assert_eq!(notes.len(), self.input_notes().num_notes());

        Ok(self.clone())
    }
}
