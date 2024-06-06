// MOCK HOST
// ================================================================================================

use alloc::{string::ToString, vec::Vec};

use miden_lib::transaction::{TransactionEvent, TransactionKernel};
use miden_objects::{
    accounts::{
        account_id::testing::ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN, Account,
        AccountDelta, AccountStub, AccountVaultDelta,
    },
    notes::Note,
    testing::{
        account::MockAccountType,
        account_code::mock_account_code,
        block::{MockChain, MockChainBuilder},
        build_dummy_tx_program,
        notes::{mock_notes, AssetPreservationStatus},
    },
    transaction::{
        ExecutedTransaction, OutputNote, OutputNotes, TransactionArgs, TransactionInputs,
        TransactionOutputs,
    },
    FieldElement,
};
use vm_processor::{
    AdviceExtractor, AdviceInjector, AdviceInputs, AdviceProvider, AdviceSource, ContextId,
    ExecutionError, Felt, Host, HostResponse, MemAdviceProvider, ProcessState, Word,
};

use super::account_procs::AccountProcedureIndexMap;

/// This is very similar to the TransactionHost in miden-tx. The differences include:
/// - We do not track account delta here.
/// - There is special handling of EMPTY_DIGEST in account procedure index map.
/// - This host uses `MemAdviceProvider` which is instantiated from the passed in advice inputs.
pub struct MockHost {
    adv_provider: MemAdviceProvider,
    acct_procedure_index_map: AccountProcedureIndexMap,
}

impl MockHost {
    /// Returns a new [MockHost] instance with the provided [AdviceInputs].
    pub fn new(account: AccountStub, advice_inputs: AdviceInputs) -> Self {
        let adv_provider: MemAdviceProvider = advice_inputs.into();
        let proc_index_map = AccountProcedureIndexMap::new(account.code_root(), &adv_provider);
        Self {
            adv_provider,
            acct_procedure_index_map: proc_index_map,
        }
    }

    /// Consumes `self` and returns the advice provider and account vault delta.
    pub fn into_parts(self) -> (MemAdviceProvider, AccountVaultDelta) {
        (self.adv_provider, AccountVaultDelta::default())
    }

    // EVENT HANDLERS
    // --------------------------------------------------------------------------------------------

    fn on_push_account_procedure_index<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), ExecutionError> {
        let proc_idx = self
            .acct_procedure_index_map
            .get_proc_index(process)
            .map_err(|err| ExecutionError::EventError(err.to_string()))?;
        self.adv_provider.push_stack(AdviceSource::Value(proc_idx.into()))?;
        Ok(())
    }
}

impl Host for MockHost {
    fn get_advice<S: ProcessState>(
        &mut self,
        process: &S,
        extractor: AdviceExtractor,
    ) -> Result<HostResponse, ExecutionError> {
        self.adv_provider.get_advice(process, &extractor)
    }

    fn set_advice<S: ProcessState>(
        &mut self,
        process: &S,
        injector: AdviceInjector,
    ) -> Result<HostResponse, ExecutionError> {
        self.adv_provider.set_advice(process, &injector)
    }

    fn on_event<S: ProcessState>(
        &mut self,
        process: &S,
        event_id: u32,
    ) -> Result<HostResponse, ExecutionError> {
        let event = TransactionEvent::try_from(event_id)
            .map_err(|err| ExecutionError::EventError(err.to_string()))?;

        if process.ctx() != ContextId::root() {
            return Err(ExecutionError::EventError(format!(
                "{event} event can only be emitted from the root context"
            )));
        }

        match event {
            TransactionEvent::AccountPushProcedureIndex => {
                self.on_push_account_procedure_index(process)
            },
            _ => Ok(()),
        }?;

        Ok(HostResponse::None)
    }
}

pub fn mock_inputs(
    account_type: MockAccountType,
    asset_preservation: AssetPreservationStatus,
) -> (TransactionInputs, TransactionArgs) {
    mock_inputs_with_account_seed(account_type, asset_preservation, None, None)
}

pub fn mock_inputs_with_account_seed(
    account_type: MockAccountType,
    asset_preservation: AssetPreservationStatus,
    account_seed: Option<Word>,
    consumed_notes_from: Option<Vec<Note>>,
) -> (TransactionInputs, TransactionArgs) {
    let assembler = &TransactionKernel::assembler();
    let account = match account_type {
        MockAccountType::StandardNew { account_id } => {
            let code = mock_account_code(assembler);
            Account::new_dummy(account_id, Felt::ZERO, code)
        },
        MockAccountType::StandardExisting => Account::new_dummy(
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
            Felt::ONE,
            mock_account_code(assembler),
        ),
        MockAccountType::FungibleFaucet { acct_id, nonce, empty_reserved_slot } => {
            Account::dummy_fungible_faucet(acct_id, nonce, empty_reserved_slot, assembler)
        },
        MockAccountType::NonFungibleFaucet { acct_id, nonce, empty_reserved_slot } => {
            Account::dummy_non_fungible_faucet(acct_id, nonce, empty_reserved_slot, assembler)
        },
    };

    let (mut input_notes, output_notes) = mock_notes(assembler, &asset_preservation);

    if let Some(ref notes) = consumed_notes_from {
        input_notes = notes.to_vec();
    }

    let mock_chain = test_chain(input_notes.clone());
    let tx_input_notes: Vec<_> = input_notes.iter().map(|n| n.id()).collect();
    let tx_inputs = mock_chain.get_transaction_inputs(account, account_seed, &tx_input_notes);

    let output_notes = output_notes.into_iter().filter_map(|n| match n {
        OutputNote::Full(note) => Some(note),
        OutputNote::Partial(_) => None,
        OutputNote::Header(_) => None,
    });
    let mut tx_args = TransactionArgs::default();
    tx_args.extend_expected_output_notes(output_notes);

    (tx_inputs, tx_args)
}

pub fn mock_executed_tx(asset_preservation: AssetPreservationStatus) -> ExecutedTransaction {
    let assembler = TransactionKernel::assembler();

    let initial_account = Account::new_dummy(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        Felt::ONE,
        mock_account_code(&assembler),
    );

    // nonce incremented by 1
    let final_account = Account::new_dummy(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        Felt::new(2),
        initial_account.code().clone(),
    );

    let (input_notes, output_notes) = mock_notes(&assembler, &asset_preservation);
    let mock_chain = test_chain(input_notes.clone());
    let input_note_ids: Vec<_> = input_notes.iter().map(|n| n.id()).collect();
    let tx_inputs = mock_chain.get_transaction_inputs(initial_account, None, &input_note_ids);

    let mut tx_args: TransactionArgs = TransactionArgs::default();
    for note in &output_notes {
        if let OutputNote::Full(note) = note {
            tx_args.add_expected_output_note(note);
        }
    }

    let tx_outputs = TransactionOutputs {
        account: final_account.into(),
        output_notes: OutputNotes::new(output_notes).unwrap(),
    };

    let program = build_dummy_tx_program();
    let account_delta = AccountDelta::default();
    let advice_witness = AdviceInputs::default();

    ExecutedTransaction::new(program, tx_inputs, tx_outputs, account_delta, tx_args, advice_witness)
}

pub fn test_chain(consumed_notes: Vec<Note>) -> MockChain {
    let mut mock_chain = MockChainBuilder::new().notes(consumed_notes).build();
    // Create 3 other blocks to land on 4 blocks
    mock_chain.seal_block();
    mock_chain.seal_block();
    mock_chain.seal_block();

    mock_chain
}

pub fn mock_inputs_with_existing(
    account_type: MockAccountType,
    asset_preservation: AssetPreservationStatus,
    account: Option<Account>,
    consumed_notes_from: Option<Vec<Note>>,
) -> (TransactionInputs, Vec<OutputNote>) {
    let assembler = &TransactionKernel::assembler();
    let account = if let Some(acc) = account {
        acc
    } else {
        match account_type {
            MockAccountType::StandardNew { account_id } => {
                let code = mock_account_code(assembler);
                Account::new_dummy(account_id, Felt::ZERO, code)
            },
            MockAccountType::StandardExisting => Account::new_dummy(
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
                Felt::ONE,
                mock_account_code(assembler),
            ),
            MockAccountType::FungibleFaucet { acct_id, nonce, empty_reserved_slot } => {
                Account::dummy_fungible_faucet(acct_id, nonce, empty_reserved_slot, assembler)
            },
            MockAccountType::NonFungibleFaucet { acct_id, nonce, empty_reserved_slot } => {
                Account::dummy_non_fungible_faucet(acct_id, nonce, empty_reserved_slot, assembler)
            },
        }
    };

    let (mock_chain, created_notes) = if let Some(ref notes) = consumed_notes_from {
        (test_chain(notes.clone()), vec![])
    } else {
        let (consumed_notes, created_notes) = mock_notes(assembler, &asset_preservation);
        (test_chain(consumed_notes), created_notes)
    };

    let tx_input_notes: Vec<_> = mock_chain.available_notes().iter().map(|n| n.id()).collect();
    let tx_inputs = mock_chain.get_transaction_inputs(account, None, &tx_input_notes);

    (tx_inputs, created_notes)
}
