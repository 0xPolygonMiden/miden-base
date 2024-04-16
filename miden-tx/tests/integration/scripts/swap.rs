use miden_lib::notes::{create_swap_note, utils::build_p2id_recipient};
use miden_objects::{
    accounts::{
        Account, AccountId, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN, ACCOUNT_ID_SENDER,
    },
    assembly::ProgramAst,
    assets::{Asset, AssetVault, FungibleAsset, NonFungibleAsset, NonFungibleAssetDetails},
    crypto::{rand::RpoRandomCoin},
    notes::{NoteAssets, NoteEnvelope, NoteExecutionMode, NoteId, NoteMetadata, NoteTag, NoteType},
    transaction::TransactionArgs,
    Felt, ZERO,
};
use miden_tx::TransactionExecutor;
use mock::mock::account::DEFAULT_AUTH_SCRIPT;

use crate::{
    get_account_with_default_account_code, get_new_key_pair_with_advice_map,
    prove_and_verify_transaction, MockDataStore,
};

#[test]
fn prove_swap_script() {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let non_fungible_asset: Asset = NonFungibleAsset::new(
        &NonFungibleAssetDetails::new(faucet_id_2, vec![1, 2, 3, 4]).unwrap(),
    )
    .unwrap()
    .into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN).unwrap();
    let (target_pub_key, target_sk_felt) = get_new_key_pair_with_advice_map();
    let target_account = get_account_with_default_account_code(
        target_account_id,
        target_pub_key,
        Some(non_fungible_asset),
    );

    // Create the note containing the SWAP script
    let (note, repay_serial_num) = create_swap_note(
        sender_account_id,
        fungible_asset,
        non_fungible_asset,
        NoteType::Public,
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store =
        MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note.clone()]));

    let mut executor = TransactionExecutor::new();
    executor.load_account(target_account_id, &data_store).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let tx_script_code = ProgramAst::parse(DEFAULT_AUTH_SCRIPT).unwrap();
    let tx_script_target = executor
        .compile_tx_script(tx_script_code.clone(), vec![(target_pub_key, target_sk_felt)], vec![])
        .unwrap();
    let tx_args_target =
        TransactionArgs::new(Some(tx_script_target), None, data_store.tx_args.advice_map().clone());

    let executed_transaction = executor
        .execute_transaction(target_account_id, block_ref, &note_ids, tx_args_target, &data_store)
        .expect("Transaction consuming swap note failed");

    // Prove, serialize/deserialize and verify the transaction
    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    // target account vault delta
    let target_account_after: Account = Account::new(
        target_account.id(),
        AssetVault::new(&[fungible_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );

    // Check that the target account has received the asset from the note
    assert_eq!(executed_transaction.final_account().hash(), target_account_after.hash());

    // Check if only one `Note` has been created
    assert_eq!(executed_transaction.output_notes().num_notes(), 1);

    // Check if the created `Note` is what we expect
    let recipient = build_p2id_recipient(sender_account_id, repay_serial_num).unwrap();
    let tag = NoteTag::from_account_id(sender_account_id, NoteExecutionMode::Local).unwrap();
    let note_metadata =
        NoteMetadata::new(target_account_id, NoteType::OffChain, tag, ZERO).unwrap();
    let assets = NoteAssets::new(vec![non_fungible_asset]).unwrap();
    let note_id = NoteId::new(recipient, assets.commitment());

    let created_note = executed_transaction.output_notes().get_note(0);
    assert_eq!(
        NoteEnvelope::from(created_note),
        NoteEnvelope::new(note_id, note_metadata).unwrap()
    );
}
