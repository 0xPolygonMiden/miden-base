use anyhow::Context;
use miden_crypto::merkle::LeafIndex;
use miden_objects::{Felt, FieldElement, MIN_PROOF_SECURITY_LEVEL};

use crate::{
    tests::utils::{generate_batch, setup_chain_without_auth, TestSetup},
    LocalBlockProver,
};

#[test]
fn proven_block_compute_new_tree_roots() -> anyhow::Result<()> {
    let TestSetup { mut chain, mut txs, .. } = setup_chain_without_auth(4);

    let tx0 = txs.remove(&0).unwrap();
    let tx1 = txs.remove(&1).unwrap();
    let tx2 = txs.remove(&2).unwrap();
    let tx3 = txs.remove(&3).unwrap();

    let batch0 = generate_batch(&mut chain, [tx0, tx1].to_vec());
    let batch1 = generate_batch(&mut chain, [tx2, tx3].to_vec());

    let proposed_block =
        chain.propose_block([batch0, batch1]).context("failed to propose block")?;

    // Compute control nullifier root on the full SMT.
    let mut current_nullifier_tree = chain.nullifiers().clone();
    for nullifier in proposed_block.nullifiers().keys() {
        current_nullifier_tree.insert(
            nullifier.inner(),
            [Felt::from(proposed_block.block_num()), Felt::ZERO, Felt::ZERO, Felt::ZERO],
        );
    }

    // Compute control account root on the full SimpleSmt.
    let mut current_account_tree = chain.accounts().clone();
    for (account_id, witness) in proposed_block.updated_accounts() {
        current_account_tree
            .insert(LeafIndex::from(*account_id), *witness.final_state_commitment());
    }

    let proven_block = LocalBlockProver::new(MIN_PROOF_SECURITY_LEVEL)
        .prove_without_verification(proposed_block)
        .context("failed to prove proposed block")?;

    assert_eq!(proven_block.header().nullifier_root(), current_nullifier_tree.root());
    assert_eq!(proven_block.header().account_root(), current_account_tree.root());

    // The Mmr in MockChain adds a new block after it is sealed, so at this point the chain contains
    // block1 and has length 2.
    // This means the chain root of the mock chain must match the chain root of the ChainMmr with
    // chain length 1 when the prev block (block1) is added.
    assert_eq!(proven_block.header().chain_root(), chain.block_chain().peaks().hash_peaks());

    Ok(())
}
