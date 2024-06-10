// TRANSACTION CONTEXT
// ================================================================================================

use alloc::vec::Vec;

use miden_lib::transaction::{ToTransactionKernelInputs, TransactionKernel};
use miden_objects::{
    accounts::{
        account_id::testing::ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN, Account,
        AccountCode,
    },
    assembly::Assembler,
    notes::{Note, NoteId},
    testing::{
        account::MockAccountType,
        block::MockChain,
        notes::{mock_notes, AssetPreservationStatus},
    },
    transaction::{
        InputNote, InputNotes, OutputNote, PreparedTransaction, TransactionArgs, TransactionInputs,
    },
    FieldElement,
};
use vm_processor::{AdviceInputs, ExecutionError, Felt, Process, Word};

use super::{executor::CodeExecutor, utils::create_test_chain, MockHost};

pub struct TransactionContext {
    mock_chain: MockChain,
    expected_output_notes: Vec<Note>,
    tx_args: TransactionArgs,
    tx_inputs: TransactionInputs,
}

impl TransactionContext {
    pub fn execute_code(&self, code: &str) -> Result<Process<MockHost>, ExecutionError> {
        self.execute_with_inputs(code, AdviceInputs::default())
    }

    pub fn execute_with_inputs(
        &self,
        code: &str,
        inputs: AdviceInputs,
    ) -> Result<Process<MockHost>, ExecutionError> {
        let tx = self.get_prepared_transaction(code);
        let (stack_inputs, mut advice_inputs) = tx.get_kernel_inputs();
        advice_inputs.extend(inputs);

        CodeExecutor::new(MockHost::new(tx.account().into(), advice_inputs))
            .stack_inputs(stack_inputs)
            .run(code)
    }

    pub fn execute_transaction(
        &self,
        tx: &PreparedTransaction,
    ) -> Result<Process<MockHost>, ExecutionError> {
        let (stack_inputs, advice_inputs) = tx.get_kernel_inputs();

        CodeExecutor::new(MockHost::new(tx.account().into(), advice_inputs))
            .stack_inputs(stack_inputs)
            .execute_program(tx.program().clone())
    }

    pub fn get_prepared_transaction(&self, code: &str) -> PreparedTransaction {
        let assembler = TransactionKernel::assembler();
        let program = assembler.compile(code).unwrap();
        PreparedTransaction::new(program, self.tx_inputs.clone(), self.tx_args.clone())
    }

    pub fn account(&self) -> &Account {
        self.tx_inputs.account()
    }

    pub fn account_seed(&self) -> Option<Word> {
        self.tx_inputs.account_seed()
    }

    pub fn expected_output_notes(&self) -> &[Note] {
        &self.expected_output_notes
    }

    pub fn mock_chain_mut(&mut self) -> &mut MockChain {
        &mut self.mock_chain
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

    pub fn into_parts(self) -> (MockChain, Vec<Note>, TransactionArgs, TransactionInputs) {
        (self.mock_chain, self.expected_output_notes, self.tx_args, self.tx_inputs)
    }
}

// TRANSACTION CONTEXT BUILDER
// ================================================================================================

pub struct TransactionContextBuilder {
    assembler: Assembler,
    account: Account,
    account_seed: Option<Word>,
    created_notes: Vec<Note>,
    expected_output_notes: Vec<Note>,
    tx_args: TransactionArgs,
}

impl TransactionContextBuilder {
    pub fn new(account: Account, assembler: Assembler) -> Self {
        let tx_args = TransactionArgs::default();
        Self {
            account,
            account_seed: None,
            assembler,
            created_notes: Vec::new(),
            expected_output_notes: Vec::new(),
            tx_args,
        }
    }

    pub fn with_acc_type(account_type: MockAccountType) -> Self {
        let assembler = TransactionKernel::assembler().with_debug_mode(true);
        let account = match account_type {
            MockAccountType::StandardNew { account_id } => {
                let code = AccountCode::mock_wallet(&assembler);
                Account::mock(account_id, Felt::ZERO, code)
            },
            MockAccountType::StandardExisting => Account::mock(
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
                Felt::ONE,
                AccountCode::mock_wallet(&assembler),
            ),
            MockAccountType::FungibleFaucet { acct_id, nonce, empty_reserved_slot } => {
                Account::mock_fungible_faucet(acct_id, nonce, empty_reserved_slot, &assembler)
            },
            MockAccountType::NonFungibleFaucet { acct_id, nonce, empty_reserved_slot } => {
                Account::mock_non_fungible_faucet(acct_id, nonce, empty_reserved_slot, &assembler)
            },
        };

        Self {
            account,
            assembler,
            account_seed: None,
            created_notes: Vec::new(),
            expected_output_notes: Vec::new(),
            tx_args: TransactionArgs::default(),
        }
    }

    pub fn account_seed(mut self, account_seed: Word) -> Self {
        self.account_seed = Some(account_seed);
        self
    }

    pub fn notes(mut self, created_notes: Vec<Note>) -> Self {
        self.created_notes.extend(created_notes);
        self
    }

    pub fn tx_args(mut self, tx_args: TransactionArgs) -> Self {
        self.tx_args = tx_args;
        self
    }

    pub fn expected_notes(mut self, output_notes: Vec<OutputNote>) -> Self {
        let output_notes = output_notes.into_iter().filter_map(|n| match n {
            OutputNote::Full(note) => Some(note),
            OutputNote::Partial(_) => None,
            OutputNote::Header(_) => None,
        });
        self.expected_output_notes.extend(output_notes);
        self
    }

    /// Populates input and expected notes with the results from [mock_notes()]
    pub fn with_mock_notes(self, asset_preservation: AssetPreservationStatus) -> Self {
        let (notes, output_notes) = mock_notes(&self.assembler, &asset_preservation);
        self.notes(notes).expected_notes(output_notes)
    }

    pub fn build(mut self) -> TransactionContext {
        let mock_chain = create_test_chain(self.created_notes.clone());
        let input_note_ids: Vec<NoteId> =
            mock_chain.available_notes().iter().map(|n| n.id()).collect();
        let tx_inputs = mock_chain.get_transaction_inputs(
            self.account.clone(),
            self.account_seed,
            &input_note_ids,
        );

        self.tx_args.extend_expected_output_notes(self.expected_output_notes.clone());
        TransactionContext {
            mock_chain,
            expected_output_notes: self.expected_output_notes,
            tx_args: self.tx_args,
            tx_inputs,
        }
    }
}
