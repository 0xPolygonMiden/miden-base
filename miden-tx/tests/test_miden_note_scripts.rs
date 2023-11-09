use miden_lib::notes::{create_note, Script};
use miden_objects::{
    accounts::{Account, AccountId, AccountVault},
    assembly::ProgramAst,
    assets::{Asset, FungibleAsset},
    utils::collections::Vec,
    Felt, StarkField,
};
use miden_tx::TransactionExecutor;
use mock::constants::{
    ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
    ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN, ACCOUNT_ID_SENDER,
};

mod common;
use common::{
    get_account_with_default_account_code, get_new_key_pair_with_advice_map, MockDataStore,
};

// P2ID TESTS
// ===============================================================================================
// We test the Pay to ID script. So we create a note that can only be consumed by the target
// account.
#[test]
fn test_p2id_script() {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let (target_pub_key, target_sk_pk_felt) = get_new_key_pair_with_advice_map();
    let target_account =
        get_account_with_default_account_code(target_account_id, target_pub_key.clone(), None);

    // Create the note
    let p2id_script = Script::P2ID {
        target: target_account_id,
    };
    let note = create_note(
        p2id_script,
        vec![fungible_asset],
        sender_account_id,
        None,
        [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)],
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store =
        MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note.clone()]), None);

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(target_account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins =
        data_store.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    let tx_script_code = ProgramAst::parse(
        format!(
            "
        use.miden::eoa::basic->auth_tx

        begin
            call.auth_tx::auth_tx_rpo_falcon512
        end
        "
        )
        .as_str(),
    )
    .unwrap();
    let tx_script_target = executor
        .compile_tx_script(
            tx_script_code.clone(),
            vec![(target_pub_key, target_sk_pk_felt)],
            vec![],
        )
        .unwrap();

    // Execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(target_account_id, block_ref, &note_origins, Some(tx_script_target))
        .unwrap();

    // vault delta
    let target_account_after: Account = Account::new(
        target_account.id(),
        AccountVault::new(&vec![fungible_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );
    assert!(transaction_result.final_account_hash() == target_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Failure)
    // --------------------------------------------------------------------------------------------
    // A "malicious" account tries to consume the note, we expect an error

    let malicious_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN + 1).unwrap();
    let (malicious_pub_key, malicious_keypair_felt) = get_new_key_pair_with_advice_map();
    let malicious_account = get_account_with_default_account_code(
        malicious_account_id,
        malicious_pub_key.clone(),
        None,
    );

    let data_store_malicious_account =
        MockDataStore::with_existing(Some(malicious_account), Some(vec![note]), None);
    let mut executor_2 = TransactionExecutor::new(data_store_malicious_account.clone());
    executor_2.load_account(malicious_account_id).unwrap();
    let tx_script_malicious = executor
        .compile_tx_script(
            tx_script_code,
            vec![(malicious_pub_key, malicious_keypair_felt)],
            vec![],
        )
        .unwrap();

    let block_ref = data_store_malicious_account.block_header.block_num().as_int() as u32;
    let note_origins = data_store_malicious_account
        .notes
        .iter()
        .map(|note| note.origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_2 = executor_2.execute_transaction(
        malicious_account_id,
        block_ref,
        &note_origins,
        Some(tx_script_malicious),
    );

    // Check that we got the expected result - TransactionExecutorError
    assert!(transaction_result_2.is_err());
}

/// We test the Pay to script with 2 assets to test the loop inside the script.
/// So we create a note containing two assets that can only be consumed by the target account.
#[test]
fn test_p2id_script_multiple_assets() {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id, 123).unwrap().into();

    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap();
    let fungible_asset_2: Asset = FungibleAsset::new(faucet_id_2, 456).unwrap().into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let (target_pub_key, target_keypair_felt) = get_new_key_pair_with_advice_map();
    let target_account =
        get_account_with_default_account_code(target_account_id, target_pub_key.clone(), None);

    // Create the note
    let p2id_script = Script::P2ID {
        target: target_account_id,
    };
    let note = create_note(
        p2id_script,
        vec![fungible_asset_1, fungible_asset_2],
        sender_account_id,
        None,
        [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)],
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store =
        MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note.clone()]), None);

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(target_account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins =
        data_store.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    let tx_script_code = ProgramAst::parse(
        format!(
            "
        use.miden::eoa::basic->auth_tx

        begin
            call.auth_tx::auth_tx_rpo_falcon512
        end
        "
        )
        .as_str(),
    )
    .unwrap();
    let tx_script_target = executor
        .compile_tx_script(
            tx_script_code.clone(),
            vec![(target_pub_key, target_keypair_felt)],
            vec![],
        )
        .unwrap();

    // Execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(target_account_id, block_ref, &note_origins, Some(tx_script_target))
        .unwrap();

    // vault delta
    let target_account_after: Account = Account::new(
        target_account.id(),
        AccountVault::new(&vec![fungible_asset_1, fungible_asset_2]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );
    assert!(transaction_result.final_account_hash() == target_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Failure)
    // --------------------------------------------------------------------------------------------
    // A "malicious" account tries to consume the note, we expect an error

    let malicious_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN + 1).unwrap();
    let (malicious_pub_key, malicious_keypair_felt) = get_new_key_pair_with_advice_map();
    let malicious_account = get_account_with_default_account_code(
        malicious_account_id,
        malicious_pub_key.clone(),
        None,
    );

    let data_store_malicious_account =
        MockDataStore::with_existing(Some(malicious_account), Some(vec![note]), None);
    let mut executor_2 = TransactionExecutor::new(data_store_malicious_account.clone());
    executor_2.load_account(malicious_account_id).unwrap();
    let tx_script_malicious = executor
        .compile_tx_script(
            tx_script_code.clone(),
            vec![(malicious_pub_key, malicious_keypair_felt)],
            vec![],
        )
        .unwrap();

    let block_ref = data_store_malicious_account.block_header.block_num().as_int() as u32;
    let note_origins = data_store_malicious_account
        .notes
        .iter()
        .map(|note| note.origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_2 = executor_2.execute_transaction(
        malicious_account_id,
        block_ref,
        &note_origins,
        Some(tx_script_malicious),
    );

    // Check that we got the expected result - TransactionExecutorError
    assert!(transaction_result_2.is_err());
}

// P2IDR TESTS
// ===============================================================================================
// We want to test the Pay to ID Reclaim script, which is a script that allows the user
// to provide a block height to the P2ID script. Before the block height is reached,
// the note can only be consumed by the target account. After the block height is reached,
// the note can also be consumed (reclaimed) by the sender account.
#[test]
fn test_p2idr_script() {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    // Create sender and target and malicious account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
    let (sender_pub_key, sender_keypair_felt) = get_new_key_pair_with_advice_map();
    let sender_account =
        get_account_with_default_account_code(sender_account_id, sender_pub_key.clone(), None);

    // Now create the target account
    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let (target_pub_key, target_keypair_felt) = get_new_key_pair_with_advice_map();
    let target_account =
        get_account_with_default_account_code(target_account_id, target_pub_key.clone(), None);

    // Now create the malicious account
    let malicious_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN + 1).unwrap();
    let (malicious_pub_key, malicious_keypair_felt) = get_new_key_pair_with_advice_map();
    let malicious_account = get_account_with_default_account_code(
        malicious_account_id,
        malicious_pub_key.clone(),
        None,
    );

    // --------------------------------------------------------------------------------------------
    // Create notes
    // Create the reclaim block height (Note: Current block height is 4)
    let reclaim_block_height_in_time = 5_u32;
    let reclaim_block_height_reclaimable = 3_u32;

    // Create the notes with the P2IDR script
    let p2idr_script_in_time = Script::P2IDR {
        target: target_account_id,
        recall_height: reclaim_block_height_in_time,
    };
    let note_in_time = create_note(
        p2idr_script_in_time,
        vec![fungible_asset],
        sender_account_id,
        None,
        [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)],
    )
    .unwrap();

    let p2idr_script_reclaimable = Script::P2IDR {
        target: target_account_id,
        recall_height: reclaim_block_height_reclaimable,
    };
    let note_reclaimable = create_note(
        p2idr_script_reclaimable,
        vec![fungible_asset],
        sender_account_id,
        None,
        [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)],
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
        None,
    );
    let mut executor_1 = TransactionExecutor::new(data_store_1.clone());

    executor_1.load_account(target_account_id).unwrap();

    let block_ref_1 = data_store_1.block_header.block_num().as_int() as u32;
    let note_origins =
        data_store_1.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    let tx_script_code = ProgramAst::parse(
        format!(
            "
        use.miden::eoa::basic->auth_tx

        begin
            call.auth_tx::auth_tx_rpo_falcon512
        end
        "
        )
        .as_str(),
    )
    .unwrap();
    let tx_script_target = executor_1
        .compile_tx_script(
            tx_script_code.clone(),
            vec![(target_pub_key, target_keypair_felt)],
            vec![],
        )
        .unwrap();

    // Execute the transaction and get the witness
    let transaction_result_1 = executor_1
        .execute_transaction(
            target_account_id,
            block_ref_1,
            &note_origins,
            Some(tx_script_target.clone()),
        )
        .unwrap();

    // Assert that the target_account received the funds and the nonce increased by 1
    let target_account_after: Account = Account::new(
        target_account_id,
        AccountVault::new(&vec![fungible_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );
    assert!(transaction_result_1.final_account_hash() == target_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Case "in time" - Sender Account Execution Failure)
    // --------------------------------------------------------------------------------------------
    let data_store_2 = MockDataStore::with_existing(
        Some(sender_account.clone()),
        Some(vec![note_in_time.clone()]),
        None,
    );
    let mut executor_2 = TransactionExecutor::new(data_store_2.clone());
    executor_2.load_account(sender_account_id).unwrap();
    let tx_script_sender = executor_2
        .compile_tx_script(
            tx_script_code.clone(),
            vec![(sender_pub_key, sender_keypair_felt)],
            vec![],
        )
        .unwrap();

    let block_ref_2 = data_store_2.block_header.block_num().as_int() as u32;
    let note_origins_2 =
        data_store_2.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_2 = executor_2.execute_transaction(
        sender_account_id,
        block_ref_2,
        &note_origins_2,
        Some(tx_script_sender.clone()),
    );

    // Check that we got the expected result - TransactionExecutorError and not TransactionResult
    // Second transaction should not work (sender consumes too early), we expect an error
    assert!(transaction_result_2.is_err());

    // CONSTRUCT AND EXECUTE TX (Case "in time" - Malicious Target Account Failure)
    // --------------------------------------------------------------------------------------------
    let data_store_3 = MockDataStore::with_existing(
        Some(malicious_account.clone()),
        Some(vec![note_in_time.clone()]),
        None,
    );
    let mut executor_3 = TransactionExecutor::new(data_store_3.clone());
    executor_3.load_account(malicious_account_id).unwrap();
    let tx_script_malicious = executor_3
        .compile_tx_script(
            tx_script_code,
            vec![(malicious_pub_key, malicious_keypair_felt)],
            vec![],
        )
        .unwrap();

    let block_ref_3 = data_store_3.block_header.block_num().as_int() as u32;
    let note_origins_3 =
        data_store_3.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_3 = executor_3.execute_transaction(
        malicious_account_id,
        block_ref_3,
        &note_origins_3,
        Some(tx_script_malicious.clone()),
    );

    // Check that we got the expected result - TransactionExecutorError and not TransactionResult
    // Third transaction should not work (malicious account can never consume), we expect an error
    assert!(transaction_result_3.is_err());

    // CONSTRUCT AND EXECUTE TX (Case "reclaimable" - Execution Target Account Success)
    // --------------------------------------------------------------------------------------------
    let data_store_4 = MockDataStore::with_existing(
        Some(target_account.clone()),
        Some(vec![note_reclaimable.clone()]),
        None,
    );
    let mut executor_4 = TransactionExecutor::new(data_store_4.clone());
    executor_4.load_account(target_account_id).unwrap();

    let block_ref_4 = data_store_4.block_header.block_num().as_int() as u32;
    let note_origins_4 =
        data_store_4.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_4 = executor_4
        .execute_transaction(
            target_account_id,
            block_ref_4,
            &note_origins_4,
            Some(tx_script_target),
        )
        .unwrap();

    // Check that we got the expected result - TransactionResult
    // Assert that the target_account received the funds and the nonce increased by 1
    // Nonce delta
    assert!(transaction_result_4.account_delta().nonce == Some(Felt::new(2)));

    // Vault delta
    let target_account_after: Account = Account::new(
        target_account_id,
        AccountVault::new(&vec![fungible_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );
    assert!(transaction_result_4.final_account_hash() == target_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Case "too late" - Execution Sender Account Success)
    // --------------------------------------------------------------------------------------------
    let data_store_5 = MockDataStore::with_existing(
        Some(sender_account.clone()),
        Some(vec![note_reclaimable.clone()]),
        None,
    );
    let mut executor_5 = TransactionExecutor::new(data_store_5.clone());

    executor_5.load_account(sender_account_id).unwrap();

    let block_ref_5 = data_store_5.block_header.block_num().as_int() as u32;
    let note_origins =
        data_store_5.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_5 = executor_5
        .execute_transaction(sender_account_id, block_ref_5, &note_origins, Some(tx_script_sender))
        .unwrap();

    // Assert that the sender_account received the funds and the nonce increased by 1
    // Nonce delta
    assert!(transaction_result_5.account_delta().nonce == Some(Felt::new(2)));

    // Vault delta (Note: vault was empty before)
    let sender_account_after: Account = Account::new(
        sender_account_id,
        AccountVault::new(&vec![fungible_asset]).unwrap(),
        sender_account.storage().clone(),
        sender_account.code().clone(),
        Felt::new(2),
    );
    assert!(transaction_result_5.final_account_hash() == sender_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Case "too late" - Malicious Account Failure)
    // --------------------------------------------------------------------------------------------
    let data_store_6 = MockDataStore::with_existing(
        Some(malicious_account.clone()),
        Some(vec![note_reclaimable.clone()]),
        None,
    );
    let mut executor_6 = TransactionExecutor::new(data_store_6.clone());

    executor_6.load_account(malicious_account_id).unwrap();

    let block_ref_6 = data_store_6.block_header.block_num().as_int() as u32;
    let note_origins_6 =
        data_store_6.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_6 = executor_6.execute_transaction(
        malicious_account_id,
        block_ref_6,
        &note_origins_6,
        Some(tx_script_malicious),
    );

    // Check that we got the expected result - TransactionExecutorError and not TransactionResult
    // Sixth transaction should not work (malicious account can never consume), we expect an error
    assert!(transaction_result_6.is_err())
}
