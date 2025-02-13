use std::{collections::BTreeMap, vec::Vec};

use anyhow::Context;
use miden_objects::{
    account::AccountId, block::ProposedBlock,
    testing::account_id::ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, transaction::ProvenTransaction,
};

use crate::tests::utils::{
    generate_batch, generate_executed_tx, generate_fungible_asset,
    generate_tracked_note_with_asset, setup_chain_with_auth, setup_chain_without_auth,
    ProvenTransactionExt, TestSetup,
};

/// Tests that a proposed block from two batches with one transaction each can be successfully
/// built.
#[test]
fn proposed_block_basic_success() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, mut txs, .. } = setup_chain_without_auth(2);
    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();
    let proven_tx0 = txs.remove(&0).unwrap();
    let proven_tx1 = txs.remove(&1).unwrap();

    let batch0 = generate_batch(&mut chain, vec![proven_tx0.clone()]);
    let batch1 = generate_batch(&mut chain, vec![proven_tx1.clone()]);

    let batches = [batch0, batch1];
    let block_inputs = chain.get_block_inputs(&batches);

    let proposed_block = ProposedBlock::new(block_inputs.clone(), batches.to_vec()).unwrap();

    assert_eq!(proposed_block.batches(), batches);
    assert_eq!(proposed_block.block_num(), block_inputs.prev_block_header().block_num() + 1);
    let updated_accounts =
        proposed_block.updated_accounts().iter().cloned().collect::<BTreeMap<_, _>>();

    assert_eq!(updated_accounts.len(), 2);
    assert_eq!(updated_accounts[&account0.id()].transactions(), &[proven_tx0.id()]);
    assert_eq!(updated_accounts[&account1.id()].transactions(), &[proven_tx1.id()]);
    assert_eq!(
        updated_accounts[&account0.id()].final_state_commitment(),
        proven_tx0.account_update().final_state_hash()
    );
    assert_eq!(
        updated_accounts[&account1.id()].final_state_commitment(),
        proven_tx1.account_update().final_state_hash()
    );

    Ok(())
}

/// Tests that account updates are correctly aggregated into a block-level account update.
#[test]
fn proposed_block_aggregates_account_state_transition() -> anyhow::Result<()> {
    // We need authentication because we're modifying accounts with the input notes.
    let TestSetup { mut chain, mut accounts, .. } = setup_chain_with_auth(2);
    let asset = generate_fungible_asset(
        100,
        AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap(),
    );

    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();

    let note0 = generate_tracked_note_with_asset(&mut chain, account0.id(), account1.id(), asset);
    let note1 = generate_tracked_note_with_asset(&mut chain, account0.id(), account1.id(), asset);
    let note2 = generate_tracked_note_with_asset(&mut chain, account0.id(), account1.id(), asset);

    // Add notes to the chain.
    chain.seal_block(None);

    // Create three transactions on the same account that build on top of each other.
    // The MockChain only updates the account state when sealing a block, but we don't want the
    // transactions to actually be added to the chain because of unintended side effects like spent
    // nullifiers. So we create an alternative chain on which we generate the transactions, but
    // then actually use the transactions on the original chain.
    let mut alternative_chain = chain.clone();
    let executed_tx0 = generate_executed_tx(&mut alternative_chain, account1.id(), &[note0.id()]);
    alternative_chain.apply_executed_transaction(&executed_tx0);
    alternative_chain.seal_block(None);

    let executed_tx1 = generate_executed_tx(&mut alternative_chain, account1.id(), &[note1.id()]);
    alternative_chain.apply_executed_transaction(&executed_tx1);
    alternative_chain.seal_block(None);

    let executed_tx2 = generate_executed_tx(&mut alternative_chain, account1.id(), &[note2.id()]);
    alternative_chain.apply_executed_transaction(&executed_tx2);

    let [tx0, tx1, tx2] = [executed_tx0, executed_tx1, executed_tx2]
        .into_iter()
        .map(|tx| {
            ProvenTransaction::from_executed_transaction_mocked(tx, &chain.latest_block_header())
        })
        .collect::<Vec<_>>()
        .try_into()
        .expect("we should have provided three executed txs");

    let batch0 = generate_batch(&mut chain, vec![tx2.clone()]);
    let batch1 = generate_batch(&mut chain, vec![tx0.clone(), tx1.clone()]);

    let batches = vec![batch0.clone(), batch1.clone()];
    let block_inputs = chain.get_block_inputs(&batches);

    let block =
        ProposedBlock::new(block_inputs, batches).context("failed to build proposed block")?;

    assert_eq!(block.updated_accounts().len(), 1);
    let (account_id, account_update) = &block.updated_accounts()[0];
    assert_eq!(*account_id, account1.id());
    assert_eq!(
        account_update.initial_state_commitment(),
        tx0.account_update().init_state_hash()
    );
    assert_eq!(account_update.final_state_commitment(), tx2.account_update().final_state_hash());
    // The transactions that affected the account are in chronological order.
    assert_eq!(account_update.transactions(), [tx0.id(), tx1.id(), tx2.id()]);
    assert!(account_update.details().is_private());

    Ok(())
}
