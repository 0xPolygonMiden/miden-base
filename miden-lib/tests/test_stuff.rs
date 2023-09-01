pub mod common;
use common::{prepare_transaction, run_tx, MemAdviceProvider, data::ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN};
use crypto::hash::rpo::RpoDigest as Digest;
use rand::{self, SeedableRng};
use rand_chacha::ChaCha8Rng;
use vm_core::StackInputs;

use miden_objects::{
    assets::FungibleAsset,
    builder::{NoteBuilder, DEFAULT_ACCOUNT_CODE},
    mock::{Immutable, MockChain, OnChain}, AccountId,
};

#[test]
// Testing the basic Miden wallet - receiving an asset
fn test_add_asset_via_wallet() {
    // Mock data
    // We need an account and a note carrying an asset.

    let mut mock_chain = MockChain::new(ChaCha8Rng::seed_from_u64(0)).unwrap();

    // Create the faucet
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();


    // Create an asset
    let asset = FungibleAsset::new(faucet_id, 100).unwrap();

    let account_code = format!("");
    
    mock_chain
        .new_account(
            account_code,
            vec![],
            vec![],
            Immutable::No,
            OnChain::No,
        )
        .unwrap();
}
