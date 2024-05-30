use alloc::vec::Vec;

use miden_objects::{
    accounts::{
        account_id::testing::ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        testing::{
            chain::mock_chain_data, mock_account, mock_account_code, mock_fungible_faucet,
            mock_new_account, mock_non_fungible_faucet, MockAccountType,
        },
        Account, AccountDelta,
    },
    notes::Note,
    testing::build_dummy_tx_program,
    transaction::{
        ChainMmr, ExecutedTransaction, InputNote, InputNotes, OutputNote, OutputNotes,
        TransactionArgs, TransactionInputs, TransactionOutputs,
    },
    vm::AdviceInputs,
    BlockHeader, Felt, FieldElement, Word,
};

use self::notes::{mock_notes, AssetPreservationStatus};
use crate::transaction::TransactionKernel;

pub mod notes;
pub mod procedures;

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
