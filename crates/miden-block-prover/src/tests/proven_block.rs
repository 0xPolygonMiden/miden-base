use anyhow::Context;
use miden_crypto::merkle::LeafIndex;
use miden_objects::{
    block::{BlockNoteIndex, BlockNoteTree},
    Felt, FieldElement, MIN_PROOF_SECURITY_LEVEL,
};

use crate::{
    tests::utils::{
        generate_batch, generate_output_note, generate_tx_with_authenticated_notes,
        generate_untracked_note_with_output_note, setup_chain, TestSetup,
    },
    LocalBlockProver,
};

/// Tests the outputs of a proven block with transactions that consume notes, create output notes
/// and modify the account's state.
#[test]
fn proven_block_success() -> anyhow::Result<()> {
    // Setup test with notes that produce output notes, in order to test the block note tree root
    // computation.
    // --------------------------------------------------------------------------------------------

    let TestSetup { mut chain, mut accounts, .. } = setup_chain(4);

    let account0 = accounts.remove(&0).unwrap();
    let account1 = accounts.remove(&1).unwrap();
    let account2 = accounts.remove(&2).unwrap();
    let account3 = accounts.remove(&3).unwrap();

    let output_note0 = generate_output_note(account0.id(), [0; 32]);
    let output_note1 = generate_output_note(account1.id(), [1; 32]);
    let output_note2 = generate_output_note(account2.id(), [2; 32]);
    let output_note3 = generate_output_note(account3.id(), [3; 32]);

    let input_note0 = generate_untracked_note_with_output_note(account0.id(), output_note0);
    let input_note1 = generate_untracked_note_with_output_note(account1.id(), output_note1);
    let input_note2 = generate_untracked_note_with_output_note(account2.id(), output_note2);
    let input_note3 = generate_untracked_note_with_output_note(account3.id(), output_note3);

    // Add input notes to chain so we can consume them.
    chain.add_pending_note(input_note0.clone());
    chain.add_pending_note(input_note1.clone());
    chain.add_pending_note(input_note2.clone());
    chain.add_pending_note(input_note3.clone());
    chain.seal_block(None);

    let tx0 = generate_tx_with_authenticated_notes(&mut chain, account0.id(), &[input_note0.id()]);
    let tx1 = generate_tx_with_authenticated_notes(&mut chain, account1.id(), &[input_note1.id()]);
    let tx2 = generate_tx_with_authenticated_notes(&mut chain, account2.id(), &[input_note2.id()]);
    let tx3 = generate_tx_with_authenticated_notes(&mut chain, account3.id(), &[input_note3.id()]);

    let batch0 = generate_batch(&mut chain, [tx0.clone(), tx1.clone()].to_vec());
    let batch1 = generate_batch(&mut chain, [tx2.clone(), tx3.clone()].to_vec());

    // Sanity check: Batches should have two output notes each.
    assert_eq!(batch0.output_notes().len(), 2);
    assert_eq!(batch1.output_notes().len(), 2);

    let proposed_block = chain
        .propose_block([batch0.clone(), batch1.clone()])
        .context("failed to propose block")?;

    // Compute expected block note tree.
    // --------------------------------------------------------------------------------------------

    let batch0_iter = batch0
        .output_notes()
        .iter()
        .enumerate()
        .map(|(note_idx_in_batch, note)| (0, note_idx_in_batch, note));
    let batch1_iter = batch1
        .output_notes()
        .iter()
        .enumerate()
        .map(|(note_idx_in_batch, note)| (1, note_idx_in_batch, note));

    let expected_note_root = BlockNoteTree::with_entries(batch0_iter.chain(batch1_iter).map(
        |(batch_idx, note_idx_in_batch, note)| {
            (BlockNoteIndex::new(batch_idx, note_idx_in_batch), note.id(), *note.metadata())
        },
    ))
    .unwrap()
    .root();

    // Compute expected nullifier root on the full SMT.
    // --------------------------------------------------------------------------------------------

    let mut expected_nullifier_tree = chain.nullifiers().clone();
    for nullifier in proposed_block.nullifiers().keys() {
        expected_nullifier_tree.insert(
            nullifier.inner(),
            [Felt::from(proposed_block.block_num()), Felt::ZERO, Felt::ZERO, Felt::ZERO],
        );
    }

    // Compute expected account root on the full SimpleSmt.
    // --------------------------------------------------------------------------------------------

    let mut expected_account_tree = chain.accounts().clone();
    for (account_id, witness) in proposed_block.updated_accounts() {
        expected_account_tree
            .insert(LeafIndex::from(*account_id), *witness.final_state_commitment());
    }

    // Prove block.
    // --------------------------------------------------------------------------------------------

    let proven_block = LocalBlockProver::new(MIN_PROOF_SECURITY_LEVEL)
        .prove_without_verification(proposed_block)
        .context("failed to prove proposed block")?;

    // Check tree/chain roots against expected values.
    // --------------------------------------------------------------------------------------------

    assert_eq!(proven_block.header().nullifier_root(), expected_nullifier_tree.root());
    assert_eq!(proven_block.header().account_root(), expected_account_tree.root());

    // The Mmr in MockChain adds a new block after it is sealed, so at this point the chain contains
    // block2 and has length 3.
    // This means the chain root of the mock chain must match the chain root of the ChainMmr with
    // chain length 2 when the prev block (block2) is added.
    assert_eq!(proven_block.header().chain_root(), chain.block_chain().peaks().hash_peaks());

    assert_eq!(proven_block.header().note_root(), expected_note_root);

    // Check input notes / nullifiers.
    // --------------------------------------------------------------------------------------------

    assert_eq!(proven_block.created_nullifiers().len(), 4);
    assert!(proven_block.created_nullifiers().contains(&input_note0.nullifier()));
    assert!(proven_block.created_nullifiers().contains(&input_note1.nullifier()));
    assert!(proven_block.created_nullifiers().contains(&input_note2.nullifier()));
    assert!(proven_block.created_nullifiers().contains(&input_note3.nullifier()));

    // Check output notes.
    // --------------------------------------------------------------------------------------------

    assert_eq!(proven_block.output_note_batches().len(), 2);
    assert_eq!(proven_block.output_note_batches()[0], batch0.output_notes());
    assert_eq!(proven_block.output_note_batches()[1], batch1.output_notes());

    // Check account updates.
    // --------------------------------------------------------------------------------------------

    // The block-level account updates should be the same as the one on transaction-level.
    for (tx, batch) in [(&tx0, &batch0), (&tx1, &batch0), (&tx2, &batch1), (&tx3, &batch1)] {
        let updated_account = tx.account_id();
        let block_account_update = proven_block
            .updated_accounts()
            .iter()
            .find(|update| update.account_id() == updated_account)
            .expect("account should have been updated in the block");

        assert_eq!(block_account_update.transactions(), [tx.id()]);
        assert_eq!(
            block_account_update.final_state_commitment(),
            batch.account_updates().get(&updated_account).unwrap().final_state_commitment()
        );
    }

    Ok(())
}
