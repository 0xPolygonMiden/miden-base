use miden_lib::notes::create_p2idr_note;
use miden_objects::{
    accounts::{
        Account, AccountId, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2, ACCOUNT_ID_SENDER,
    },
    assembly::ProgramAst,
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::rand::RpoRandomCoin,
    notes::NoteType,
    transaction::TransactionArgs,
    Felt,
};
use miden_tx::TransactionExecutor;
use mock::mock::account::DEFAULT_AUTH_SCRIPT;

use crate::{
    get_account_with_default_account_code, get_new_key_pair_with_advice_map, MockDataStore,
};

// P2IDR TESTS
// ===============================================================================================
// We want to test the Pay to ID Reclaim script, which is a script that allows the user
// to provide a block height to the P2ID script. Before the block height is reached,
// the note can only be consumed by the target account. After the block height is reached,
// the note can also be consumed (reclaimed) by the sender account.
#[test]
fn p2idr_script() {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    // Create sender and target and malicious account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
    let (sender_pub_key, sender_keypair_felt) = get_new_key_pair_with_advice_map();
    let sender_account =
        get_account_with_default_account_code(sender_account_id, sender_pub_key, None);

    // Now create the target account
    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let (target_pub_key, target_keypair_felt) = get_new_key_pair_with_advice_map();
    let target_account =
        get_account_with_default_account_code(target_account_id, target_pub_key, None);

    // Now create the malicious account
    let malicious_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2).unwrap();
    let (malicious_pub_key, malicious_keypair_felt) = get_new_key_pair_with_advice_map();
    let malicious_account =
        get_account_with_default_account_code(malicious_account_id, malicious_pub_key, None);

    // --------------------------------------------------------------------------------------------
    // Create notes
    // Create the reclaim block height (Note: Current block height is 4)
    let reclaim_block_height_in_time = 5_u32;
    let reclaim_block_height_reclaimable = 3_u32;

    // Create the notes with the P2IDR script
    // Create the note_in_time
    let note_in_time = create_p2idr_note(
        sender_account_id,
        target_account_id,
        vec![fungible_asset],
        NoteType::Public,
        reclaim_block_height_in_time,
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // Create the reclaimable_note
    let note_reclaimable = create_p2idr_note(
        sender_account_id,
        target_account_id,
        vec![fungible_asset],
        NoteType::Public,
        reclaim_block_height_reclaimable,
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // --------------------------------------------------------------------------------------------
    // We have two cases:
    //  Case "in time": block height is 4, reclaim block height is 5. Only the target account can consume the note.
    //  Case "reclaimable": block height is 4, reclaim block height is 3. Target and sender account can consume the note.
    //  The malicious account should never be able to consume the note.
    // --------------------------------------------------------------------------------------------
    // CONSTRUCT AND EXECUTE TX (Case "in time" - Target Account Execution Success)
    // --------------------------------------------------------------------------------------------
    let data_store_1 = MockDataStore::with_existing(
        Some(target_account.clone()),
        Some(vec![note_in_time.clone()]),
    );
    let mut executor = TransactionExecutor::new();

    executor.load_account(target_account_id, &data_store_1).unwrap();

    let block_ref_1 = data_store_1.block_header.block_num();
    let note_ids = data_store_1.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let tx_script_code = ProgramAst::parse(DEFAULT_AUTH_SCRIPT).unwrap();
    let tx_script_target = executor
        .compile_tx_script(
            tx_script_code.clone(),
            vec![(target_pub_key, target_keypair_felt)],
            vec![],
        )
        .unwrap();
    let tx_args_target = TransactionArgs::with_tx_script(tx_script_target);

    // Execute the transaction and get the witness
    let executed_transaction_1 = executor
        .execute_transaction(
            target_account_id,
            block_ref_1,
            &note_ids,
            tx_args_target.clone(),
            &data_store_1,
        )
        .unwrap();

    // Assert that the target_account received the funds and the nonce increased by 1
    let target_account_after: Account = Account::new(
        target_account_id,
        AssetVault::new(&[fungible_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );
    assert_eq!(executed_transaction_1.final_account().hash(), target_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Case "in time" - Sender Account Execution Failure)
    // --------------------------------------------------------------------------------------------
    let data_store_2 = MockDataStore::with_existing(
        Some(sender_account.clone()),
        Some(vec![note_in_time.clone()]),
    );

    executor.load_account(sender_account_id, &data_store_2).unwrap();
    let tx_script_sender = executor
        .compile_tx_script(
            tx_script_code.clone(),
            vec![(sender_pub_key, sender_keypair_felt)],
            vec![],
        )
        .unwrap();
    let tx_args_sender = TransactionArgs::with_tx_script(tx_script_sender);

    let block_ref_2 = data_store_2.block_header.block_num();
    let note_ids_2 = data_store_2.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let executed_transaction_2 = executor.execute_transaction(
        sender_account_id,
        block_ref_2,
        &note_ids_2,
        tx_args_sender.clone(),
        &data_store_2,
    );

    // Check that we got the expected result - TransactionExecutorError and not ExecutedTransaction
    // Second transaction should not work (sender consumes too early), we expect an error
    assert!(executed_transaction_2.is_err());

    // CONSTRUCT AND EXECUTE TX (Case "in time" - Malicious Target Account Failure)
    // --------------------------------------------------------------------------------------------
    let data_store_3 = MockDataStore::with_existing(
        Some(malicious_account.clone()),
        Some(vec![note_in_time.clone()]),
    );
    executor.load_account(malicious_account_id, &data_store_3).unwrap();
    let tx_script_malicious = executor
        .compile_tx_script(
            tx_script_code,
            vec![(malicious_pub_key, malicious_keypair_felt)],
            vec![],
        )
        .unwrap();
    let tx_args_malicious = TransactionArgs::with_tx_script(tx_script_malicious);

    let block_ref_3 = data_store_3.block_header.block_num();
    let note_ids_3 = data_store_3.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let executed_transaction_3 = executor.execute_transaction(
        malicious_account_id,
        block_ref_3,
        &note_ids_3,
        tx_args_malicious.clone(),
        &data_store_3,
    );

    // Check that we got the expected result - TransactionExecutorError and not ExecutedTransaction
    // Third transaction should not work (malicious account can never consume), we expect an error
    assert!(executed_transaction_3.is_err());

    // CONSTRUCT AND EXECUTE TX (Case "reclaimable" - Execution Target Account Success)
    // --------------------------------------------------------------------------------------------
    let data_store_4 = MockDataStore::with_existing(
        Some(target_account.clone()),
        Some(vec![note_reclaimable.clone()]),
    );
    executor.load_account(target_account_id, &data_store_4).unwrap();

    let block_ref_4 = data_store_4.block_header.block_num();
    let note_ids_4 = data_store_4.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let executed_transaction_4 = executor
        .execute_transaction(
            target_account_id,
            block_ref_4,
            &note_ids_4,
            tx_args_target,
            &data_store_4,
        )
        .unwrap();

    // Check that we got the expected result - ExecutedTransaction
    // Assert that the target_account received the funds and the nonce increased by 1
    // Nonce delta
    assert_eq!(executed_transaction_4.account_delta().nonce(), Some(Felt::new(2)));

    // Vault delta
    let target_account_after: Account = Account::new(
        target_account_id,
        AssetVault::new(&[fungible_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );
    assert_eq!(executed_transaction_4.final_account().hash(), target_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Case "too late" - Execution Sender Account Success)
    // --------------------------------------------------------------------------------------------
    let data_store_5 = MockDataStore::with_existing(
        Some(sender_account.clone()),
        Some(vec![note_reclaimable.clone()]),
    );
    executor.load_account(sender_account_id, &data_store_5).unwrap();

    let block_ref_5 = data_store_5.block_header.block_num();
    let note_ids_5 = data_store_5.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let executed_transaction_5 = executor
        .execute_transaction(
            sender_account_id,
            block_ref_5,
            &note_ids_5,
            tx_args_sender,
            &data_store_5,
        )
        .unwrap();

    // Assert that the sender_account received the funds and the nonce increased by 1
    // Nonce delta
    assert_eq!(executed_transaction_5.account_delta().nonce(), Some(Felt::new(2)));

    // Vault delta (Note: vault was empty before)
    let sender_account_after: Account = Account::new(
        sender_account_id,
        AssetVault::new(&[fungible_asset]).unwrap(),
        sender_account.storage().clone(),
        sender_account.code().clone(),
        Felt::new(2),
    );
    assert_eq!(executed_transaction_5.final_account().hash(), sender_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Case "too late" - Malicious Account Failure)
    // --------------------------------------------------------------------------------------------
    let data_store_6 = MockDataStore::with_existing(
        Some(malicious_account.clone()),
        Some(vec![note_reclaimable.clone()]),
    );

    executor.load_account(malicious_account_id, &data_store_6).unwrap();

    let block_ref_6 = data_store_6.block_header.block_num();
    let note_ids_6 = data_store_6.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let executed_transaction_6 = executor.execute_transaction(
        malicious_account_id,
        block_ref_6,
        &note_ids_6,
        tx_args_malicious,
        &data_store_6,
    );

    // Check that we got the expected result - TransactionExecutorError and not ExecutedTransaction
    // Sixth transaction should not work (malicious account can never consume), we expect an error
    assert!(executed_transaction_6.is_err())
}
