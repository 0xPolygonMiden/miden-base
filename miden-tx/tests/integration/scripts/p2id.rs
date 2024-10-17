use miden_lib::{notes::create_p2id_note, transaction::TransactionKernel};
use miden_objects::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
            ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2, ACCOUNT_ID_SENDER,
        },
        Account,
    },
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::rand::RpoRandomCoin,
    notes::NoteType,
    testing::prepare_word,
    transaction::{OutputNote, TransactionScript},
    Felt,
};
use miden_tx::testing::mock_chain::MockChain;

use crate::prove_and_verify_transaction;

/// We test the Pay to script with 2 assets to test the loop inside the script.
/// So we create a note containing two assets that can only be consumed by the target account.
#[test]
fn p2id_script_multiple_assets() {
    let mut mock_chain = MockChain::new();

    // Create assets
    let fungible_asset_1: Asset = FungibleAsset::mock(123);
    let fungible_asset_2: Asset =
        FungibleAsset::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2.try_into().unwrap(), 456)
            .unwrap()
            .into();

    // Create sender and target account
    let sender_account = mock_chain.add_new_wallet(vec![]);
    let target_account = mock_chain.add_existing_wallet(vec![]);

    // Create the note
    let note = mock_chain
        .add_p2id_note(
            sender_account.id(),
            target_account.id(),
            &[fungible_asset_1, fungible_asset_2],
            NoteType::Public,
            None,
        )
        .unwrap();

    mock_chain.seal_block(None);

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    // Execute the transaction and get the witness
    let executed_transaction = mock_chain
        .build_tx_context(target_account.id(), &[note.id()], &[])
        .build()
        .execute()
        .unwrap();

    // vault delta
    let target_account_after: Account = Account::from_parts(
        target_account.id(),
        AssetVault::new(&[fungible_asset_1, fungible_asset_2]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );

    assert_eq!(executed_transaction.final_account().hash(), target_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Failure)
    // --------------------------------------------------------------------------------------------
    // A "malicious" account tries to consume the note, we expect an error (not the correct target)

    let malicious_account = mock_chain.add_existing_wallet(vec![]);
    mock_chain.seal_block(None);

    // Execute the transaction and get the witness
    let executed_transaction_2 = mock_chain
        .build_tx_context(malicious_account.id(), &[], &[note])
        .build()
        .execute();

    // Check that we got the expected result - TransactionExecutorError
    assert!(executed_transaction_2.is_err());
}

// /// Consumes an existing note with a new account
// #[test]
// fn prove_consume_note_with_new_account() {
//     let (mut target_account, seed, falcon_auth) = create_new_account();
//     let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
//     let fungible_asset_1: Asset = FungibleAsset::new(faucet_id, 123).unwrap().into();

//     // Create the note
//     let note = create_p2id_note(
//         ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap(),
//         target_account.id(),
//         vec![fungible_asset_1],
//         NoteType::Public,
//         Felt::new(0),
//         &mut RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
//     )
//     .unwrap();

//     let tx_context = TransactionContextBuilder::new(target_account.clone())
//         .account_seed(Some(seed))
//         .input_notes(vec![note.clone()])
//         .build();

//     assert!(target_account.is_new());

//     let executor = TransactionExecutor::new(tx_context.clone(), Some(falcon_auth));

//     let block_ref = tx_context.tx_inputs().block_header().block_num();
//     let note_ids = tx_context
//         .tx_inputs()
//         .input_notes()
//         .iter()
//         .map(|note| note.id())
//         .collect::<Vec<_>>();

//     let tx_script_target = build_default_auth_script();
//     let tx_args_target = TransactionArgs::with_tx_script(tx_script_target);

//     // Execute the transaction and get the witness
//     let executed_transaction = executor
//         .execute_transaction(target_account.id(), block_ref, &note_ids, tx_args_target)
//         .unwrap();

//     // Account delta
//     target_account.apply_delta(executed_transaction.account_delta()).unwrap();
//     assert!(!target_account.is_new());

//     assert!(prove_and_verify_transaction(executed_transaction).is_ok());
// }

/// Consumes two existing notes (with an asset from a faucet for a combined total of 123 tokens)
/// with a basic account
#[test]
fn prove_consume_multiple_notes() {
    let mut mock_chain = MockChain::new();
    let mut account = mock_chain.add_existing_wallet(vec![]);

    let fungible_asset_1: Asset =
        FungibleAsset::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap(), 100)
            .unwrap()
            .into();
    let fungible_asset_2: Asset =
        FungibleAsset::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap(), 23)
            .unwrap()
            .into();

    let note_1 = mock_chain
        .add_p2id_note(
            ACCOUNT_ID_SENDER.try_into().unwrap(),
            account.id(),
            &[fungible_asset_1],
            NoteType::Private,
            None,
        )
        .unwrap();
    let note_2 = mock_chain
        .add_p2id_note(
            ACCOUNT_ID_SENDER.try_into().unwrap(),
            account.id(),
            &[fungible_asset_2],
            NoteType::Private,
            None,
        )
        .unwrap();

    mock_chain.seal_block(None);

    let tx_context = mock_chain
        .build_tx_context(account.id(), &[note_1.id(), note_2.id()], &[])
        .build();

    let executed_transaction = tx_context.execute().unwrap();

    account.apply_delta(executed_transaction.account_delta()).unwrap();
    let resulting_asset = account.vault().assets().next().unwrap();
    if let Asset::Fungible(asset) = resulting_asset {
        assert_eq!(asset.amount(), 123u64);
    } else {
        panic!("Resulting asset should be fungible");
    }

    assert!(prove_and_verify_transaction(executed_transaction).is_ok());
}

/// Consumes two existing notes and creates two other notes in the same transaction
#[test]
fn test_create_consume_multiple_notes() {
    let mut mock_chain = MockChain::new();
    let mut account = mock_chain.add_existing_wallet(vec![FungibleAsset::mock(20)]);

    let input_note_faucet_id = ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN;
    let input_note_asset_1: Asset =
        FungibleAsset::new(input_note_faucet_id.try_into().unwrap(), 11).unwrap().into();

    let input_note_asset_2: Asset =
        FungibleAsset::new(input_note_faucet_id.try_into().unwrap(), 100)
            .unwrap()
            .into();

    let input_note_1 = mock_chain
        .add_p2id_note(
            ACCOUNT_ID_SENDER.try_into().unwrap(),
            account.id(),
            &[input_note_asset_1],
            NoteType::Private,
            None,
        )
        .unwrap();

    let input_note_2 = mock_chain
        .add_p2id_note(
            ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2.try_into().unwrap(),
            account.id(),
            &[input_note_asset_2],
            NoteType::Private,
            None,
        )
        .unwrap();

    mock_chain.seal_block(None);

    let output_note_1 = create_p2id_note(
        account.id(),
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2.try_into().unwrap(),
        vec![FungibleAsset::mock(10)],
        NoteType::Public,
        Felt::new(0),
        &mut RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    let output_note_2 = create_p2id_note(
        account.id(),
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.try_into().unwrap(),
        vec![FungibleAsset::mock(5)],
        NoteType::Public,
        Felt::new(0),
        &mut RpoRandomCoin::new([Felt::new(4), Felt::new(3), Felt::new(2), Felt::new(1)]),
    )
    .unwrap();

    let tx_script_src = &format!(
        "
            begin
                push.{recipient_1}
                push.{note_execution_hint_1}
                push.{note_type_1}
                push.0              # aux
                push.{tag_1}
                call.::miden::contracts::wallets::basic::create_note

                push.{asset_1}
                call.::miden::contracts::wallets::basic::move_asset_to_note
                dropw dropw dropw dropw

                push.{recipient_2}
                push.{note_execution_hint_2}
                push.{note_type_2}
                push.0              # aux
                push.{tag_2}
                call.::miden::contracts::wallets::basic::create_note

                push.{asset_2}
                call.::miden::contracts::wallets::basic::move_asset_to_note
                dropw dropw dropw dropw
                call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
            end
            ",
        recipient_1 = prepare_word(&output_note_1.recipient().digest()),
        note_type_1 = NoteType::Public as u8,
        tag_1 = Felt::new(output_note_1.metadata().tag().into()),
        asset_1 = prepare_word(&FungibleAsset::mock(10).into()),
        note_execution_hint_1 = Felt::from(output_note_1.metadata().execution_hint()),
        recipient_2 = prepare_word(&output_note_2.recipient().digest()),
        note_type_2 = NoteType::Public as u8,
        tag_2 = Felt::new(output_note_2.metadata().tag().into()),
        asset_2 = prepare_word(&FungibleAsset::mock(5).into()),
        note_execution_hint_2 = Felt::from(output_note_2.metadata().execution_hint())
    );

    let tx_script =
        TransactionScript::compile(tx_script_src, vec![], TransactionKernel::testing_assembler())
            .unwrap();

    let tx_context = mock_chain
        .build_tx_context(account.id(), &[input_note_1.id(), input_note_2.id()], &[])
        .expected_notes(vec![OutputNote::Full(output_note_1), OutputNote::Full(output_note_2)])
        .tx_script(tx_script)
        .build();

    let executed_transaction = tx_context.execute().unwrap();

    assert_eq!(executed_transaction.output_notes().num_notes(), 2);

    account.apply_delta(executed_transaction.account_delta()).unwrap();
    for asset in account.vault().assets() {
        if u64::from(asset.faucet_id()) == input_note_faucet_id {
            assert!(asset.unwrap_fungible().amount() == 111);
        } else if u64::from(asset.faucet_id()) == ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN {
            assert!(asset.unwrap_fungible().amount() == 5);
        }
    }
}
