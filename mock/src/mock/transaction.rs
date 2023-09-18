use super::{
    account::{
        mock_account, mock_fungible_faucet, mock_new_account, mock_non_fungible_faucet,
        MockAccountType,
    },
    block::mock_block_header,
    chain::mock_chain_data,
    notes::mock_notes,
    notes::AssetPreservationStatus,
};
use miden_lib::assembler::assembler;
use miden_objects::{
    accounts::Account, notes::Note, transaction::ExecutedTransaction, utils::collections::Vec,
    BlockHeader, ChainMmr, Felt, FieldElement,
};

pub fn mock_inputs(
    account_type: MockAccountType,
    asset_preservation: AssetPreservationStatus,
) -> (Account, BlockHeader, ChainMmr, Vec<Note>) {
    // Create assembler and assembler context
    let assembler = assembler();

    // Create an account with storage items

    let account = match account_type {
        MockAccountType::StandardNew => mock_new_account(&assembler),
        MockAccountType::StandardExisting => mock_account(Felt::ONE, None, &assembler),
        MockAccountType::FungibleFaucet(acct_id) => mock_fungible_faucet(acct_id, &assembler),
        MockAccountType::NonFungibleFaucet => mock_non_fungible_faucet(&assembler),
    };

    // mock notes
    let (mut consumed_notes, _created_notes) = mock_notes(&assembler, &asset_preservation);

    // Chain data
    let chain_mmr: ChainMmr = mock_chain_data(&mut consumed_notes);

    // Block header
    let block_header = mock_block_header(
        Felt::new(4),
        Some(chain_mmr.mmr().accumulator().hash_peaks().into()),
        None,
        &[account.clone()],
    );

    // Transaction inputs
    (account, block_header, chain_mmr, consumed_notes)
}

pub fn mock_inputs_with_existing(
    account_type: MockAccountType,
    asset_preservation: AssetPreservationStatus,
    account: Option<Account>,
    consumed_notes_from: Option<Vec<Note>>,
) -> (Account, BlockHeader, ChainMmr, Vec<Note>) {
    // Create assembler and assembler context
    let assembler = assembler();

    // Create an account with storage items

    let account = match account_type {
        MockAccountType::StandardNew => mock_new_account(&assembler),
        MockAccountType::StandardExisting => {
            account.unwrap_or(mock_account(Felt::ONE, None, &assembler))
        }
        MockAccountType::FungibleFaucet(acct_id) => {
            account.unwrap_or(mock_fungible_faucet(acct_id, &assembler))
        }
        MockAccountType::NonFungibleFaucet => mock_non_fungible_faucet(&assembler),
    };

    let (mut consumed_notes, _created_notes) = mock_notes(&assembler, &asset_preservation);
    if let Some(ref notes) = consumed_notes_from {
        consumed_notes = notes.to_vec();
    }

    // Chain data
    let chain_mmr: ChainMmr = mock_chain_data(&mut consumed_notes);

    // Block header
    let block_header = mock_block_header(
        Felt::new(4),
        Some(chain_mmr.mmr().accumulator().hash_peaks().into()),
        None,
        &[account.clone()],
    );

    // Transaction inputs
    (account, block_header, chain_mmr, consumed_notes)
}

pub fn mock_executed_tx(asset_preservation: AssetPreservationStatus) -> ExecutedTransaction {
    // Create assembler and assembler context
    let assembler = assembler();

    // Initial Account
    let initial_account = mock_account(Felt::ONE, None, &assembler);

    // Finial Account (nonce incremented by 1)
    let final_account =
        mock_account(Felt::new(2), Some(initial_account.code().clone()), &assembler);

    // mock notes
    let (mut consumed_notes, created_notes) = mock_notes(&assembler, &asset_preservation);

    // Chain data
    let chain_mmr: ChainMmr = mock_chain_data(&mut consumed_notes);

    // Block header
    let block_header = mock_block_header(
        Felt::new(4),
        Some(chain_mmr.mmr().accumulator().hash_peaks().into()),
        None,
        &[initial_account.clone()],
    );

    // Executed Transaction
    ExecutedTransaction::new(
        initial_account,
        None,
        final_account,
        consumed_notes,
        created_notes,
        None,
        block_header,
        chain_mmr,
    )
    .unwrap()
}
