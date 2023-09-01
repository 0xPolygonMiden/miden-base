pub mod common;
use common::{prepare_transaction, run_tx, MemAdviceProvider};
use crypto::hash::rpo::RpoDigest as Digest;
use rand::{self, SeedableRng};
use rand_chacha::ChaCha8Rng;
use vm_core::StackInputs;

use miden_objects::{
    assets::FungibleAsset,
    builder::{NoteBuilder, DEFAULT_ACCOUNT_CODE},
    mock::{Immutable, MockChain, OnChain},
};

#[test]
// Testing the basic Miden wallet - receiving an asset
fn test_add_asset_via_wallet() {
    // Mock data
    // We need an account and a note carrying an asset.

    let mut mock_chain = MockChain::new(ChaCha8Rng::seed_from_u64(0)).unwrap();

    // Create the faucet
    let faucet_id = mock_chain
        .new_fungible_faucet(OnChain::Yes, DEFAULT_ACCOUNT_CODE, Digest::default())
        .unwrap();

    // Create an asset
    let asset = FungibleAsset::new(faucet_id, 100).unwrap();

    // Create the account
    mock_chain
        .new_account(
            include_str!("../asm/sat/account.masm"),
            vec![],
            vec![],
            Immutable::No,
            OnChain::No,
        )
        .unwrap();

    // Create the note
    let note_script: String = format!(
        "
    use.miden::sat::note
    use.miden::wallets::basic->wallet
    use.miden::eoa::basic->authentication

    # add the asset
    begin
        exec.note::get_assets drop
        mem_loadw
        exec.wallet::receive_asset
        exec.authentication::auth_tx
    end
    "
    );

    let note = NoteBuilder::new(faucet_id, ChaCha8Rng::seed_from_u64(1))
        .add_asset(asset.into())
        .code(note_script)
        .build()
        .unwrap();
    mock_chain.add_note(note.clone()).unwrap();

    // Seal the block
    let block_header = mock_chain.seal_block().unwrap();

    // FIX: change prepare_transaction to accept references
    let transaction = prepare_transaction(
        mock_chain.account_mut(0).clone(),
        None,
        block_header,
        mock_chain.chain().clone(),
        vec![note],
        &"",
        "",
        None,
        None,
    );

    let _process = run_tx(
        transaction.tx_program().clone(),
        StackInputs::from(transaction.stack_inputs()),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    )
    .unwrap();
}
