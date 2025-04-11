#[cfg(feature = "async")]
use alloc::boxed::Box;
use alloc::{collections::BTreeSet, rc::Rc, sync::Arc, vec::Vec};

use builder::MockAuthenticator;
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    account::{Account, AccountId},
    assembly::Assembler,
    block::{BlockHeader, BlockNumber},
    note::Note,
    transaction::{
        ChainMmr, ExecutedTransaction, InputNote, InputNotes, TransactionArgs, TransactionInputs,
    },
};
use rand_chacha::ChaCha20Rng;
use vm_processor::{
    AdviceInputs, Digest, ExecutionError, MastForest, MastForestStore, Process, Word,
};
use winter_maybe_async::*;

use super::{MockHost, executor::CodeExecutor};
use crate::{
    DataStore, DataStoreError, TransactionExecutor, TransactionExecutorError, TransactionMastStore,
    auth::{BasicAuthenticator, TransactionAuthenticator},
};

mod builder;
pub use builder::TransactionContextBuilder;

// TRANSACTION CONTEXT
// ================================================================================================

/// Represents all needed data for executing a transaction, or arbitrary code.
///
/// It implements [DataStore], so transactions may be executed with
/// [TransactionExecutor](crate::TransactionExecutor)
pub struct TransactionContext {
    expected_output_notes: Vec<Note>,
    tx_args: TransactionArgs,
    tx_inputs: TransactionInputs,
    mast_store: TransactionMastStore,
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

        let test_lib = TransactionKernel::kernel_as_library();

        let program = self
            .assembler
            .clone()
            .with_debug_mode(true)
            .assemble_program(code)
            .expect("compilation of the provided code failed");

        let mast_store = Rc::new(TransactionMastStore::new());

        mast_store.insert(program.mast_forest().clone());
        mast_store.insert(test_lib.mast_forest().clone());
        mast_store.load_transaction_code(self.account().code(), self.input_notes(), &self.tx_args);

        CodeExecutor::new(MockHost::new(
            self.tx_inputs.account().into(),
            advice_inputs,
            mast_store,
            self.tx_args
                .foreign_accounts()
                .iter()
                .map(|acc| acc.account_code().commitment())
                .collect(),
        ))
        .stack_inputs(stack_inputs)
        .execute_program(program)
    }

    /// Executes the transaction through a [TransactionExecutor]
    #[allow(clippy::arc_with_non_send_sync)]
    #[maybe_async]
    pub fn execute(self) -> Result<ExecutedTransaction, TransactionExecutorError> {
        let account_id = self.account().id();
        let block_num = self.tx_inputs().block_header().block_num();
        let notes = self.tx_inputs().input_notes().clone();
        let tx_args = self.tx_args().clone();

        let authenticator = self
            .authenticator()
            .cloned()
            .map(|auth| Arc::new(auth) as Arc<dyn TransactionAuthenticator>);

        let tx_executor = TransactionExecutor::new(Arc::new(self), authenticator);

        maybe_await!(tx_executor.execute_transaction(account_id, block_num, notes, tx_args))
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

    pub fn authenticator(&self) -> Option<&BasicAuthenticator<ChaCha20Rng>> {
        self.authenticator.as_ref()
    }
}

#[maybe_async_trait]
impl DataStore for TransactionContext {
    #[maybe_async]
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        _ref_blocks: BTreeSet<BlockNumber>,
    ) -> Result<(Account, Option<Word>, BlockHeader, ChainMmr), DataStoreError> {
        assert_eq!(account_id, self.account().id());
        let (account, seed, header, mmr, _) = self.tx_inputs.clone().into_parts().clone();

        Ok((account, seed, header, mmr))
    }
}

impl MastForestStore for TransactionContext {
    fn get(&self, procedure_hash: &Digest) -> Option<Arc<MastForest>> {
        self.mast_store.get(procedure_hash)
    }
}
