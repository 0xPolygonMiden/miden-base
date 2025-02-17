use anyhow::Context;
use assert_matches::assert_matches;
use miden_objects::block::ProposedBlock;

use crate::{
    tests::utils::{
        generate_batch, generate_executed_tx_with_authenticated_notes, generate_tracked_note,
        setup_chain, TestSetup,
    },
    LocalBlockProver, ProvenBlockError,
};

/// Tests the outputs of a proven block with transactions that consume notes, create output notes
/// and modify the account's state.
#[test]
fn proven_block_fails_on_stale_account_or_nullifier_witnesses() -> anyhow::Result<()> {
    // Setup test with notes that produce output notes, in order to test the block note tree root
    // computation.
    // --------------------------------------------------------------------------------------------

    let TestSetup { mut chain, mut accounts, mut txs, .. } = setup_chain(4);

    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();

    let note = generate_tracked_note(&mut chain, account1.id(), account0.id());
    // Add note to chain.
    chain.seal_block(None);

    let tx0 =
        generate_executed_tx_with_authenticated_notes(&mut chain, account0.id(), &[note.id()]);
    let tx1 = txs.remove(&1).unwrap();
    let tx2 = txs.remove(&2).unwrap();

    let batch1 = generate_batch(&mut chain, vec![tx1, tx2]);
    let batches = vec![batch1];
    let mut stale_block_inputs = chain.get_block_inputs(&batches);

    let account_root0 = chain.accounts().root();
    let nullifier_root0 = chain.nullifiers().root();

    // Apply the executed tx and seal a block. This invalidates the block inputs we've just fetched.
    chain.apply_executed_transaction(&tx0);
    chain.seal_block(None);

    let valid_block_inputs = chain.get_block_inputs(&batches);

    // Sanity check: This test requires that the tree roots change with the last sealed block so the
    // previously fetched block inputs become invalid.
    assert_ne!(chain.accounts().root(), account_root0);
    assert_ne!(chain.nullifiers().root(), nullifier_root0);

    // Account tree root mismatch.
    // --------------------------------------------------------------------------------------------

    // Make the block inputs invalid by using the stale account witnesses.
    let mut invalid_account_tree_block_inputs = valid_block_inputs.clone();
    core::mem::swap(
        invalid_account_tree_block_inputs.account_witnesses_mut(),
        stale_block_inputs.account_witnesses_mut(),
    );

    let proposed_block0 = ProposedBlock::new(invalid_account_tree_block_inputs, batches.clone())
        .context("failed to propose block 0")?;

    let error = LocalBlockProver::new(0)
        .prove_without_verification(proposed_block0)
        .unwrap_err();

    assert_matches!(
        error,
        ProvenBlockError::StaleAccountTreeRoot {
            prev_block_account_root,
            ..
        } if prev_block_account_root == valid_block_inputs.prev_block_header().account_root()
    );

    // Nullifier tree root mismatch.
    // --------------------------------------------------------------------------------------------

    // Make the block inputs invalid by using the stale nullifier witnesses.
    let mut invalid_nullifier_tree_block_inputs = valid_block_inputs.clone();
    core::mem::swap(
        invalid_nullifier_tree_block_inputs.nullifier_witnesses_mut(),
        stale_block_inputs.nullifier_witnesses_mut(),
    );

    let proposed_block1 = ProposedBlock::new(invalid_nullifier_tree_block_inputs, batches)
        .context("failed to propose block 1")?;

    let error = LocalBlockProver::new(0)
        .prove_without_verification(proposed_block1)
        .unwrap_err();

    assert_matches!(
        error,
        ProvenBlockError::StaleNullifierTreeRoot {
          prev_block_nullifier_root,
          ..
        } if prev_block_nullifier_root == valid_block_inputs.prev_block_header().nullifier_root()
    );

    Ok(())
}
