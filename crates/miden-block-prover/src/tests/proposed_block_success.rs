use std::collections::BTreeMap;

use miden_objects::block::ProposedBlock;

use crate::tests::utils::{generate_batch, setup_chain_without_auth, TestSetup};

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
