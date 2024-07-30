use alloc::vec::Vec;

use miden_crypto::merkle::SimpleSmt;
use vm_core::Felt;
use vm_processor::Digest;
use winter_rand_utils::{rand_array, rand_value};

use crate::{accounts::Account, BlockHeader, ACCOUNT_TREE_DEPTH};

impl BlockHeader {
    /// Creates a mock block. The account tree is formed from the provided `accounts`,
    /// and the chain root and note root are set to the provided `chain_root` and `note_root`
    /// values respectively.
    ///
    /// For non-WASM targets, the remaining header values are initialized randomly. For WASM
    /// targets, values are initialized to [Default::default()]
    pub fn mock(
        block_num: u32,
        chain_root: Option<Digest>,
        note_root: Option<Digest>,
        accounts: &[Account],
    ) -> Self {
        let acct_db = SimpleSmt::<ACCOUNT_TREE_DEPTH>::with_leaves(
            accounts
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
        let acct_root = acct_db.root();

        #[cfg(not(target_family = "wasm"))]
        let (prev_hash, chain_root, nullifier_root, note_root, tx_hash, proof_hash, timestamp) = {
            let prev_hash = rand_array().into();
            let chain_root = chain_root.unwrap_or(rand_array().into());
            let nullifier_root = rand_array().into();
            let note_root = note_root.unwrap_or(rand_array().into());
            let tx_hash = rand_array().into();
            let proof_hash = rand_array().into();
            let timestamp = rand_value();

            (prev_hash, chain_root, nullifier_root, note_root, tx_hash, proof_hash, timestamp)
        };

        #[cfg(target_family = "wasm")]
        let (prev_hash, chain_root, nullifier_root, note_root, tx_hash, proof_hash, timestamp) =
            Default::default();

        BlockHeader::new(
            0,
            prev_hash,
            block_num,
            chain_root,
            acct_root,
            nullifier_root,
            note_root,
            tx_hash,
            proof_hash,
            timestamp,
        )
    }
}
