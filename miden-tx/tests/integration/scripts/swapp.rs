use miden_objects::notes::NoteExecutionMode;
use std::collections::BTreeMap;

use miden_lib::{notes::create_swapp_note, transaction::TransactionKernel};
use miden_objects::{
    accounts::Account,
    assets::AssetVault,
    crypto::rand::RpoRandomCoin,
    notes::{NoteAssets, NoteExecutionHint, NoteHeader, NoteId, NoteMetadata, NoteTag, NoteType},
    testing::account_code::DEFAULT_AUTH_SCRIPT,
    transaction::{TransactionArgs, TransactionScript},
    Felt, ZERO,
};
use miden_tx::testing::mock_chain::{Auth, MockChain};

#[test]
fn test_swapp_script_full_swap() {
    // Setup
    // --------------------------------------------------------------------------------------------
    let mut chain = MockChain::new();

    // create assets
    let faucet_1 = chain.add_existing_faucet(Auth::NoAuth, "BTC", 10);
    let faucet_2 = chain.add_existing_faucet(Auth::NoAuth, "ETH", 10);

    let offered_asset = faucet_1.mint(10);
    let requested_asset = faucet_2.mint(10);

    // create sender and target account
    let sender_account = chain.add_new_wallet(Auth::BasicAuth, vec![offered_asset]);
    let target_account = chain.add_existing_wallet(Auth::BasicAuth, vec![requested_asset]);

    let note = create_swapp_note(
        sender_account.id(),
        offered_asset,
        requested_asset,
        NoteType::Public,
        Felt::new(27),
        &mut RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // add note to chain
    chain.add_note(note.clone());
    chain.seal_block(None);

    // EXECUTE TX
    // --------------------------------------------------------------------------------------------
    let transaction_script =
        TransactionScript::compile(DEFAULT_AUTH_SCRIPT, vec![], TransactionKernel::assembler())
            .unwrap();

    let mut tx_context = chain
        .build_tx_context(target_account.id())
        .tx_script(transaction_script.clone())
        .build();

    let note_args = [Felt::new(10), Felt::new(0), Felt::new(0), Felt::new(0)];

    let note_args_map = BTreeMap::from([(note.id(), note_args)]);

    let tx_args = TransactionArgs::new(
        Some(transaction_script),
        Some(note_args_map),
        tx_context.tx_args().advice_inputs().clone().map,
    );

    tx_context.set_tx_args(tx_args);

    let executed_transaction = tx_context.execute().unwrap();

    // target account vault delta
    let target_account_after: Account = Account::from_parts(
        target_account.id(),
        AssetVault::new(&[offered_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );

    // Check that the target account has received the asset from the note
    assert_eq!(executed_transaction.final_account().hash(), target_account_after.hash());

    // Check if only one `Note` has been created
    assert_eq!(executed_transaction.output_notes().num_notes(), 1);

    // Check if the output `Note` is what we expect
    let recipient = executed_transaction.output_notes().get_note(0).recipient_digest().unwrap();
    let tag = NoteTag::from_account_id(sender_account.id(), NoteExecutionMode::Local).unwrap();
    let note_metadata = NoteMetadata::new(
        target_account.id(),
        NoteType::Private,
        tag,
        NoteExecutionHint::Always,
        ZERO,
    )
    .unwrap();
    let assets = NoteAssets::new(vec![requested_asset]).unwrap();
    let note_id = NoteId::new(recipient, assets.commitment());

    let output_note = executed_transaction.output_notes().get_note(0);
    assert_eq!(NoteHeader::from(output_note), NoteHeader::new(note_id, note_metadata));
}

#[test]
fn test_swapp_script_partial_swap() {
    // Setup
    // --------------------------------------------------------------------------------------------
    let mut chain = MockChain::new();

    // create assets
    let faucet_1 = chain.add_existing_faucet(Auth::NoAuth, "BTC", 1_000_000_000);
    let faucet_2 = chain.add_existing_faucet(Auth::NoAuth, "ETH", 20_000_000_000_000);

    let offered_asset = faucet_1.mint(1_000_000_000);
    let requested_asset = faucet_2.mint(20_000_000_000_000);

    // assets for note checks
    let filled_requested_asset = faucet_2.mint(7_000_000_000_000);
    let remaining_offered_asset = faucet_1.mint(650_000_000);
    let remaining_requested_asset = faucet_2.mint(13_000_000_000_000);

    // assets for target account vault after transaction
    let received_offered_asset = faucet_1.mint(350_000_000);

    // create sender and target account
    let sender_account = chain.add_new_wallet(Auth::BasicAuth, vec![offered_asset]);
    let target_account = chain.add_existing_wallet(Auth::BasicAuth, vec![requested_asset]);

    let note = create_swapp_note(
        sender_account.id(),
        offered_asset,
        requested_asset,
        NoteType::Public,
        Felt::new(27),
        &mut RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // add note to chain
    chain.add_note(note.clone());
    chain.seal_block(None);

    // EXECUTE TX
    // --------------------------------------------------------------------------------------------
    let transaction_script =
        TransactionScript::compile(DEFAULT_AUTH_SCRIPT, vec![], TransactionKernel::assembler())
            .unwrap();

    let mut tx_context = chain
        .build_tx_context(target_account.id())
        .tx_script(transaction_script.clone())
        .build();

    let note_args = [Felt::new(7_000_000_000_000), Felt::new(0), Felt::new(0), Felt::new(0)];

    let note_args_map = BTreeMap::from([(note.id(), note_args)]);

    let tx_args = TransactionArgs::new(
        Some(transaction_script),
        Some(note_args_map),
        tx_context.tx_args().advice_inputs().clone().map,
    );

    tx_context.set_tx_args(tx_args);

    let executed_transaction = tx_context.execute().unwrap();

    // target account vault delta
    let target_account_after: Account = Account::from_parts(
        target_account.id(),
        AssetVault::new(&[received_offered_asset, remaining_requested_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );

    // Check that the target account has received the asset from the note
    assert_eq!(executed_transaction.final_account().hash(), target_account_after.hash());

    // Check if only two `Note`s have been created
    assert_eq!(executed_transaction.output_notes().num_notes(), 2);

    // Check if the output `Note`s are what we expect

    // P2ID note
    let recipient = executed_transaction.output_notes().get_note(0).recipient_digest().unwrap();
    let tag = NoteTag::from_account_id(sender_account.id(), NoteExecutionMode::Local).unwrap();
    let note_metadata = NoteMetadata::new(
        target_account.id(),
        NoteType::Private,
        tag,
        NoteExecutionHint::Always,
        ZERO,
    )
    .unwrap();
    let assets = NoteAssets::new(vec![filled_requested_asset]).unwrap();
    let note_id = NoteId::new(recipient, assets.commitment());
    let p2id_output_note = executed_transaction.output_notes().get_note(0);

    assert_eq!(NoteHeader::from(p2id_output_note), NoteHeader::new(note_id, note_metadata));

    // SWAPP note
    let recipient = executed_transaction.output_notes().get_note(1).recipient_digest().unwrap();
    let tag = NoteTag::from_account_id(sender_account.id(), NoteExecutionMode::Local).unwrap();
    let note_metadata = NoteMetadata::new(
        target_account.id(),
        NoteType::Private,
        tag,
        NoteExecutionHint::Always,
        ZERO,
    )
    .unwrap();
    let assets = NoteAssets::new(vec![remaining_offered_asset]).unwrap();
    let note_id = NoteId::new(recipient, assets.commitment());
    let swapp_output_note = executed_transaction.output_notes().get_note(1);

    assert_eq!(NoteHeader::from(swapp_output_note), NoteHeader::new(note_id, note_metadata));
}
