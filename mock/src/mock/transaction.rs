use alloc::vec::Vec;

use miden_objects::{
    accounts::{Account, AccountDelta},
    notes::Note,
    transaction::{
        ChainMmr, ExecutedTransaction, InputNote, InputNotes, OutputNote, OutputNotes,
        TransactionArgs, TransactionInputs, TransactionOutputs,
    },
    BlockHeader, Felt, FieldElement,
};
use vm_processor::{AdviceInputs, Operation, Program, Word};

use super::{
    super::TransactionKernel,
    account::{
        mock_account, mock_account_code, mock_fungible_faucet, mock_new_account,
        mock_non_fungible_faucet, MockAccountType,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
    },
    block::mock_block_header,
    chain::mock_chain_data,
    notes::{mock_notes, AssetPreservationStatus},
};

pub fn mock_inputs(
    account_type: MockAccountType,
    asset_preservation: AssetPreservationStatus,
) -> TransactionInputs {
    mock_inputs_with_account_seed(account_type, asset_preservation, None)
}

pub fn mock_inputs_with_account_seed(
    account_type: MockAccountType,
    asset_preservation: AssetPreservationStatus,
    account_seed: Option<Word>,
) -> TransactionInputs {
    // Create assembler and assembler context
    let assembler = TransactionKernel::assembler();

    // Create an account with storage items
    let account = match account_type {
        MockAccountType::StandardNew => mock_new_account(&assembler),
        MockAccountType::StandardExisting => mock_account(
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
            Felt::ONE,
            mock_account_code(&assembler),
        ),
        MockAccountType::FungibleFaucet { acct_id, nonce, empty_reserved_slot } => {
            mock_fungible_faucet(acct_id, nonce, empty_reserved_slot, &assembler)
        },
        MockAccountType::NonFungibleFaucet { acct_id, nonce, empty_reserved_slot } => {
            mock_non_fungible_faucet(acct_id, nonce, empty_reserved_slot, &assembler)
        },
    };

    // mock notes
    let (input_notes, _output_notes) = mock_notes(&assembler, &asset_preservation);

    // Chain data
    let (chain_mmr, recorded_notes) = mock_chain_data(input_notes);

    // Block header
    let block_header =
        mock_block_header(4, Some(chain_mmr.peaks().hash_peaks()), None, &[account.clone()]);

    // Transaction inputs
    let input_notes = InputNotes::new(recorded_notes).unwrap();
    TransactionInputs::new(account, account_seed, block_header, chain_mmr, input_notes).unwrap()
}

pub fn mock_inputs_with_existing(
    account_type: MockAccountType,
    asset_preservation: AssetPreservationStatus,
    account: Option<Account>,
    consumed_notes_from: Option<Vec<Note>>,
) -> (Account, BlockHeader, ChainMmr, Vec<InputNote>, AdviceInputs) {
    // create auxiliary data object
    let auxiliary_data = AdviceInputs::default();

    // Create assembler and assembler context
    let assembler = TransactionKernel::assembler();

    // Create an account with storage items

    let account = match account_type {
        MockAccountType::StandardNew => mock_new_account(&assembler),
        MockAccountType::StandardExisting => account.unwrap_or(mock_account(
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
            Felt::ONE,
            mock_account_code(&assembler),
        )),
        MockAccountType::FungibleFaucet { acct_id, nonce, empty_reserved_slot } => {
            account.unwrap_or(mock_fungible_faucet(acct_id, nonce, empty_reserved_slot, &assembler))
        },
        MockAccountType::NonFungibleFaucet { acct_id, nonce, empty_reserved_slot } => {
            mock_non_fungible_faucet(acct_id, nonce, empty_reserved_slot, &assembler)
        },
    };

    let (mut consumed_notes, _created_notes) = mock_notes(&assembler, &asset_preservation);
    if let Some(ref notes) = consumed_notes_from {
        consumed_notes = notes.to_vec();
    }

    // Chain data
    let (chain_mmr, recorded_notes) = mock_chain_data(consumed_notes);

    // Block header
    let block_header =
        mock_block_header(4, Some(chain_mmr.peaks().hash_peaks()), None, &[account.clone()]);

    // Transaction inputs
    (account, block_header, chain_mmr, recorded_notes, auxiliary_data)
}

pub fn mock_executed_tx(asset_preservation: AssetPreservationStatus) -> ExecutedTransaction {
    // Create assembler and assembler context
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

    // mock notes
    let (input_notes, output_notes) = mock_notes(&assembler, &asset_preservation);

    let output_notes = output_notes.into_iter().map(OutputNote::from).collect::<Vec<_>>();

    // Chain data
    let (block_chain, input_notes) = mock_chain_data(input_notes);

    // Block header
    let block_header = mock_block_header(
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

    let tx_outputs = TransactionOutputs {
        account: final_account.into(),
        output_notes: OutputNotes::new(output_notes).unwrap(),
    };

    // dummy components
    let program = build_dummy_tx_program();
    let account_delta = AccountDelta::default();
    let advice_witness = AdviceInputs::default();
    let tx_args: TransactionArgs = TransactionArgs::default();

    // Executed Transaction
    ExecutedTransaction::new(program, tx_inputs, tx_outputs, account_delta, tx_args, advice_witness)
}

// HELPER FUNCTIONS
// ================================================================================================

fn build_dummy_tx_program() -> Program {
    let operations = vec![Operation::Push(Felt::ZERO), Operation::Drop];
    let span = miden_objects::vm::CodeBlock::new_span(operations);
    Program::new(span)
}
