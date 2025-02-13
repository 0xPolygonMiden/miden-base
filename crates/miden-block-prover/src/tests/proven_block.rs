use anyhow::Context;
use miden_crypto::merkle::LeafIndex;
use miden_objects::{
    note::NoteType, transaction::ProvenTransaction, Felt, FieldElement, MIN_PROOF_SECURITY_LEVEL,
};

use crate::{
    tests::utils::{setup_chain_without_auth, ProvenTransactionExt, TestSetup},
    LocalBlockProver,
};

#[test]
fn proven_block_compute_new_tree_roots() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut accounts, .. } = setup_chain_without_auth(2);
    let account1 = accounts.remove(&0).unwrap();
    let account2 = accounts.remove(&1).unwrap();

    let note = chain
        .add_p2id_note(account1.id(), account2.id(), &[], NoteType::Public, None)
        .context("failed to add p2id note")?;
    let block2 = chain.seal_block(None);

    let tx_context = chain.build_tx_context(account2.id(), &[note.id()], &[]).build();
    let executed_tx = tx_context.execute().context("failed to execute tx")?;
    let proven_tx =
        ProvenTransaction::from_executed_transaction_mocked(executed_tx, &block2.header());
    let batch = chain
        .propose_transaction_batch([proven_tx])
        .map(|batch| chain.prove_transaction_batch(batch))
        .context("failed to propose transaction batch")?;
    let proposed_block = chain.propose_block([batch]).context("failed to propose block")?;

    // Compute control nullifier root.
    let mut current_nullifier_tree = chain.nullifiers().clone();
    for nullifier in proposed_block.nullifiers().keys() {
        current_nullifier_tree.insert(
            nullifier.inner(),
            [Felt::from(proposed_block.block_num()), Felt::ZERO, Felt::ZERO, Felt::ZERO],
        );
    }

    // Compute control account root.
    let mut current_account_tree = chain.accounts().clone();
    for (account_id, witness) in proposed_block.updated_accounts() {
        current_account_tree
            .insert(LeafIndex::from(*account_id), *witness.final_state_commitment());
    }

    let block = LocalBlockProver::new(MIN_PROOF_SECURITY_LEVEL)
        .prove_without_verification(proposed_block)
        .context("failed to prove proposed block")?;

    assert_eq!(block.nullifier_root(), current_nullifier_tree.root());
    assert_eq!(block.account_root(), current_account_tree.root());

    // The Mmr in MockChain adds a new block after it is sealed, so at this point the chain contains
    // block2 and has length 3.
    // This means the chain root of the mock chain must match the chain root of the ChainMmr with
    // chain length 2 when the prev block (block2) is added.
    assert_eq!(block.chain_root(), chain.block_chain().peaks().hash_peaks());

    Ok(())
}
