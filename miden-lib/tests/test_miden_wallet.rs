pub mod common;

use common::{ 
    prepare_transaction, run_tx, MemAdviceProvider, data::prepare_word
};
use crypto::{hash::rpo::RpoDigest as Digest, Felt, ZERO, ONE};
use rand::{self, SeedableRng};
use rand_chacha::ChaCha8Rng;
use vm_core::StackInputs;

use miden_objects::{
    assets::FungibleAsset,
    builder::{NoteBuilder, DEFAULT_ACCOUNT_CODE},
    mock::{Immutable, MockChain, OnChain}
};

#[test]
// Testing the basic Miden wallet - receiving an asset
fn test_receive_asset_via_wallet() {
    // Create the account that owns the assets
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
<<<<<<< HEAD
        push.1
=======
>>>>>>> 3dd1d0f (rebase on MockChain PR)
    end
    "
    );

    let note = NoteBuilder::new(faucet_id, ChaCha8Rng::seed_from_u64(1))
        .add_asset(asset.into())
        .code(note_script)
        .build()
        .unwrap();
    
    mock_chain.add_note(note.clone()).unwrap();

    let account = mock_chain.account_mut(0);
    account.nonce()

    // Seal the block
    let block_header = mock_chain.seal_block().unwrap();

    // FIX: change prepare_transaction to accept references
    let transaction = prepare_transaction(
        mock_chain.account_mut(0).clone(),
        None,
        block_header,
        mock_chain.chain().clone(),
        vec![note],
        &"begin end",
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

    // ToDo: check the account has the asset
}

#[test]
// Testing the basic Miden wallet - sending an asset
fn test_send_asset_via_wallet() {
    // Mock data
    // We need an account that owns an asset

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
            vec![asset.clone().try_into().unwrap()],
            Immutable::No,
            OnChain::No,
        )
        .unwrap();

    // Seal the block
    let block_header = mock_chain.seal_block().unwrap();

    // Create the transaction
    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);

    let transaction_script: String = format!(
        "
    use.miden::wallets::basic->wallet

    begin
        push.{recipient}
        push.{tag}
        push.{asset}
        exec.wallet::send_asset
    end
        ", 
        recipient = prepare_word(&recipient),
        tag = tag,
        asset = prepare_word(&asset.try_into().unwrap())
    );

    // FIX: change prepare_transaction to accept references
    let transaction = prepare_transaction(
        mock_chain.account_mut(0).clone(),
        None,
        block_header,
        mock_chain.chain().clone(),
        vec![],
        &transaction_script,
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

    // ToDo: check the account has the asset not anymore

    // ToDo: check that there is a note with the asset
}
