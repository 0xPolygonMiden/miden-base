use miden_core::FieldElement;

use super::{
    super::{
        notes::Note, transaction::ExecutedTransaction, Account, BlockHeader, ChainMmr, Felt, Vec,
    },
    assembler, mock_account, mock_block_header, mock_chain_data, mock_fungible_faucet,
    mock_new_account, mock_non_fungible_faucet, mock_notes, AssetPreservationStatus,
    MockAccountType,
};

pub fn mock_inputs(
    account_type: MockAccountType,
    asset_preservation: AssetPreservationStatus,
    account: Option<Account>,
    consumed_notes_from: Option<Vec<Note>>,
) -> (Account, BlockHeader, ChainMmr, Vec<Note>) {
    // Create assembler and assembler context
    let mut assembler = assembler();

    // Create an account with storage items

    let account = match account_type {
        MockAccountType::StandardNew => mock_new_account(&mut assembler),
        MockAccountType::StandardExisting => {
            account.unwrap_or(mock_account(Felt::ONE, None, &mut assembler))
        }
        MockAccountType::FungibleFaucet(acct_id) => mock_fungible_faucet(acct_id, &mut assembler),
        MockAccountType::NonFungibleFaucet => mock_non_fungible_faucet(&mut assembler),
    };

    let (mut consumed_notes, _created_notes) = mock_notes(&mut assembler, asset_preservation);
    if consumed_notes_from.is_some() {
        consumed_notes = consumed_notes_from.unwrap();
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
    let mut assembler = assembler();

    // Initial Account
    let initial_account = mock_account(Felt::ONE, None, &mut assembler);

    // Finial Account (nonce incremented by 1)
    let final_account =
        mock_account(Felt::new(2), Some(initial_account.code().clone()), &mut assembler);

    // mock notes
    let (mut consumed_notes, created_notes) = mock_notes(&mut assembler, asset_preservation);

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
