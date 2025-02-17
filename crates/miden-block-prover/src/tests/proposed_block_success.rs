use std::{collections::BTreeMap, vec::Vec};

use anyhow::Context;
use miden_objects::{
    account::AccountId, block::ProposedBlock,
    testing::account_id::ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, transaction::ProvenTransaction,
};

use crate::tests::utils::{
    generate_batch, generate_executed_tx_with_authenticated_notes, generate_fungible_asset,
    generate_output_note, generate_tracked_note_with_asset, generate_tx_with_authenticated_notes,
    generate_tx_with_unauthenticated_notes, generate_untracked_note,
    generate_untracked_note_with_output_note, setup_chain, ProvenTransactionExt, TestSetup,
};

/// Tests that a proposed block from two batches with one transaction each can be successfully
/// built.
#[test]
fn proposed_block_basic_success() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, mut txs, .. } = setup_chain(2);
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
    // Each tx consumes one note.
    assert_eq!(proposed_block.nullifiers().len(), 2);
    assert!(proposed_block
        .nullifiers()
        .contains_key(&proven_tx0.input_notes().get_note(0).nullifier()));
    assert!(proposed_block
        .nullifiers()
        .contains_key(&proven_tx1.input_notes().get_note(0).nullifier()));

    // No notes were created.
    assert!(proposed_block.block_note_tree().is_empty());

    Ok(())
}

/// Tests that account updates are correctly aggregated into a block-level account update.
#[test]
fn proposed_block_aggregates_account_state_transition() -> anyhow::Result<()> {
    // We need authentication because we're modifying accounts with the input notes.
    let TestSetup { mut chain, mut accounts, .. } = setup_chain(2);
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
    let executed_tx0 = generate_executed_tx_with_authenticated_notes(
        &mut alternative_chain,
        account1.id(),
        &[note0.id()],
    );
    alternative_chain.apply_executed_transaction(&executed_tx0);
    alternative_chain.seal_block(None);

    let executed_tx1 = generate_executed_tx_with_authenticated_notes(
        &mut alternative_chain,
        account1.id(),
        &[note1.id()],
    );
    alternative_chain.apply_executed_transaction(&executed_tx1);
    alternative_chain.seal_block(None);

    let executed_tx2 = generate_executed_tx_with_authenticated_notes(
        &mut alternative_chain,
        account1.id(),
        &[note2.id()],
    );
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

/// Tests that unauthenticated notes can be authenticated when inclusion proofs are provided.
#[test]
fn proposed_block_authenticating_unauthenticated_notes() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, .. } = setup_chain(3);
    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();
    let account2 = accounts.remove(&2).unwrap();

    let note0 = generate_untracked_note(account0.id(), account1.id());
    let note1 = generate_untracked_note(account0.id(), account2.id());

    // These txs will use block1 as the reference block.
    let tx0 = generate_tx_with_unauthenticated_notes(&mut chain, account1.id(), &[note0.clone()]);
    let tx1 = generate_tx_with_unauthenticated_notes(&mut chain, account2.id(), &[note1.clone()]);

    // These batches will use block1 as the reference block.
    let batch0 = generate_batch(&mut chain, vec![tx0.clone()]);
    let batch1 = generate_batch(&mut chain, vec![tx1.clone()]);

    chain.add_pending_note(note0.clone());
    chain.add_pending_note(note1.clone());
    chain.seal_block(None);

    let batches = [batch0, batch1];
    // This block will use block2 as the reference block.
    let block_inputs = chain.get_block_inputs(&batches);

    // Sanity check: Block inputs should contain nullifiers for the unauthenticated notes since they
    // are part of the chain.
    assert!(block_inputs.nullifier_witnesses().contains_key(&note0.nullifier()));
    assert!(block_inputs.nullifier_witnesses().contains_key(&note1.nullifier()));

    let proposed_block = ProposedBlock::new(block_inputs.clone(), batches.to_vec())
        .context("failed to build proposed block")?;

    // We expect both notes to have been authenticated and therefore should be part of the
    // nullifiers of this block.
    assert_eq!(proposed_block.nullifiers().len(), 2);
    assert!(proposed_block.nullifiers().contains_key(&note0.nullifier()));
    assert!(proposed_block.nullifiers().contains_key(&note1.nullifier()));

    Ok(())
}

/// Tests that an unauthenticated note is erased when it is created in the same block.
#[test]
fn proposed_block_erasing_unauthenticated_notes() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, .. } = setup_chain(3);
    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();

    let output_note = generate_output_note(account0.id(), [10; 32]);

    let note0 = generate_untracked_note_with_output_note(account0.id(), output_note.clone());
    // Add note0 to the chain so we can consume it.
    chain.add_pending_note(note0.clone());
    chain.seal_block(None);

    let tx0 = generate_tx_with_authenticated_notes(&mut chain, account0.id(), &[note0.id()]);
    let tx1 =
        generate_tx_with_unauthenticated_notes(&mut chain, account1.id(), &[output_note.clone()]);

    assert_eq!(tx0.input_notes().num_notes(), 1);
    assert_eq!(tx0.output_notes().num_notes(), 1);
    assert_eq!(tx1.output_notes().num_notes(), 0);
    // The unauthenticated note is an input note of the tx.
    assert_eq!(tx1.input_notes().num_notes(), 1);

    assert_eq!(
        tx0.output_notes().get_note(0).id(),
        tx1.input_notes().get_note(0).header().unwrap().id()
    );

    // These batches will use block1 as the reference block.
    let batch0 = generate_batch(&mut chain, vec![tx0.clone()]);
    let batch1 = generate_batch(&mut chain, vec![tx1.clone()]);

    // Sanity check: The batches and contained transactions should have the same input notes.
    assert_eq!(batch0.input_notes(), tx0.input_notes());
    assert_eq!(batch1.input_notes(), tx1.input_notes());

    let batches = [batch0, batch1];
    // This block will use block2 as the reference block.
    let block_inputs = chain.get_block_inputs(&batches);

    let proposed_block = ProposedBlock::new(block_inputs.clone(), batches.to_vec())
        .context("failed to build proposed block")?;

    // The output note should have been erased, so we expect only note0's nullifier to be created.
    assert_eq!(proposed_block.nullifiers().len(), 1);
    assert!(proposed_block.nullifiers().contains_key(&note0.nullifier()));
    assert!(proposed_block.block_note_tree().is_empty());

    Ok(())
}
