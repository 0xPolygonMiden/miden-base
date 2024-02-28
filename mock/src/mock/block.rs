use miden_objects::{
    accounts::Account, crypto::merkle::SimpleSmt, BlockHeader, Digest, Felt, ACCOUNT_TREE_DEPTH,
    ZERO,
};
use rand_utils as rand;

pub fn mock_block_header(
    block_num: u32,
    chain_root: Option<Digest>,
    note_root: Option<Digest>,
    accts: &[Account],
) -> BlockHeader {
    let acct_db = SimpleSmt::<ACCOUNT_TREE_DEPTH>::with_leaves(
        accts
            .iter()
            .flat_map(|acct| {
                if acct.is_new() {
                    None
                } else {
                    let felt_id: Felt = acct.id().into();
                    Some((felt_id.as_int(), *acct.hash()))
                }
            })
            .collect::<Vec<_>>(),
    )
    .expect("failed to create account db");

    let prev_hash: Digest = rand::rand_array().into();
    let chain_root: Digest = chain_root.unwrap_or(rand::rand_array().into());
    let acct_root: Digest = acct_db.root();
    let nullifier_root: Digest = rand::rand_array().into();
    let note_root: Digest = note_root.unwrap_or(rand::rand_array().into());
    let batch_root: Digest = rand::rand_array().into();
    let proof_hash: Digest = rand::rand_array().into();

    BlockHeader::new(
        prev_hash,
        block_num,
        chain_root,
        acct_root,
        nullifier_root,
        note_root,
        batch_root,
        proof_hash,
        ZERO,
        rand::rand_value(),
    )
}
