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
        account::{mock_account, mock_new_account, MockAccountType},
        account_code::mock_account_code,
        build_dummy_tx_program,
        notes::{mock_notes, AssetPreservationStatus},
        storage::{mock_fungible_faucet, mock_non_fungible_faucet},
    },
    transaction::{
        ChainMmr, ExecutedTransaction, InputNote, InputNotes, OutputNote, OutputNotes,
        TransactionArgs, TransactionInputs, TransactionOutputs,
    },
    BlockHeader, FieldElement,
};
use vm_processor::{
    AdviceExtractor, AdviceInjector, AdviceInputs, AdviceProvider, AdviceSource, ContextId,
    ExecutionError, Felt, Host, HostResponse, MemAdviceProvider, ProcessState, Word,
};

use super::{account_procs::AccountProcedureIndexMap, chain_data::mock_chain_data};

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
    mock_inputs_with_account_seed(account_type, asset_preservation, None)
}

pub fn mock_inputs_with_account_seed(
    account_type: MockAccountType,
    asset_preservation: AssetPreservationStatus,
    account_seed: Option<Word>,
) -> (TransactionInputs, TransactionArgs) {
    let assembler = &TransactionKernel::assembler();
    let account = match account_type {
        MockAccountType::StandardNew => mock_new_account(assembler),
        MockAccountType::StandardExisting => mock_account(
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
            Felt::ONE,
            mock_account_code(assembler),
        ),
        MockAccountType::FungibleFaucet { acct_id, nonce, empty_reserved_slot } => {
            mock_fungible_faucet(acct_id, nonce, empty_reserved_slot, assembler)
        },
        MockAccountType::NonFungibleFaucet { acct_id, nonce, empty_reserved_slot } => {
            mock_non_fungible_faucet(acct_id, nonce, empty_reserved_slot, assembler)
        },
    };

    let (input_notes, output_notes) = mock_notes(assembler, &asset_preservation);

    let (chain_mmr, recorded_notes) = mock_chain_data(input_notes);

    let block_header =
        BlockHeader::mock(4, Some(chain_mmr.peaks().hash_peaks()), None, &[account.clone()]);

    let input_notes = InputNotes::new(recorded_notes).unwrap();
    let tx_inputs =
        TransactionInputs::new(account, account_seed, block_header, chain_mmr, input_notes)
            .unwrap();

    let output_notes = output_notes.into_iter().filter_map(|n| match n {
        OutputNote::Full(note) => Some(note),
        OutputNote::Partial(_) => None,
        OutputNote::Header(_) => None,
    });
    let mut tx_args = TransactionArgs::default();
    tx_args.extend_expected_output_notes(output_notes);

    (tx_inputs, tx_args)
}

pub fn mock_inputs_with_existing(
    account_type: MockAccountType,
    asset_preservation: AssetPreservationStatus,
    account: Option<Account>,
    consumed_notes_from: Option<Vec<Note>>,
) -> (Account, BlockHeader, ChainMmr, Vec<InputNote>, AdviceInputs, Vec<OutputNote>) {
    let auxiliary_data = AdviceInputs::default();
    let assembler = &TransactionKernel::assembler();

    let account = match account_type {
        MockAccountType::StandardNew => mock_new_account(assembler),
        MockAccountType::StandardExisting => account.unwrap_or(mock_account(
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
            Felt::ONE,
            mock_account_code(assembler),
        )),
        MockAccountType::FungibleFaucet { acct_id, nonce, empty_reserved_slot } => {
            account.unwrap_or(mock_fungible_faucet(acct_id, nonce, empty_reserved_slot, assembler))
        },
        MockAccountType::NonFungibleFaucet { acct_id, nonce, empty_reserved_slot } => {
            mock_non_fungible_faucet(acct_id, nonce, empty_reserved_slot, assembler)
        },
    };

    let (mut consumed_notes, created_notes) = mock_notes(assembler, &asset_preservation);
    if let Some(ref notes) = consumed_notes_from {
        consumed_notes = notes.to_vec();
    }

    let (chain_mmr, recorded_notes) = mock_chain_data(consumed_notes);

    let block_header =
        BlockHeader::mock(4, Some(chain_mmr.peaks().hash_peaks()), None, &[account.clone()]);

    (account, block_header, chain_mmr, recorded_notes, auxiliary_data, created_notes)
}

pub fn mock_executed_tx(asset_preservation: AssetPreservationStatus) -> ExecutedTransaction {
    let assembler = TransactionKernel::assembler();

    let initial_account = mock_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        Felt::ONE,
        mock_account_code(&assembler),
    );

    // nonce incremented by 1
    let final_account = mock_account(
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        Felt::new(2),
        initial_account.code().clone(),
    );

    let (input_notes, output_notes) = mock_notes(&assembler, &asset_preservation);
    let (block_chain, input_notes) = mock_chain_data(input_notes);

    let block_header = BlockHeader::mock(
        4,
        Some(block_chain.peaks().hash_peaks()),
        None,
        &[initial_account.clone()],
    );

    let tx_inputs = TransactionInputs::new(
        initial_account,
        None,
        block_header,
        block_chain,
        InputNotes::new(input_notes).unwrap(),
    )
    .unwrap();

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
