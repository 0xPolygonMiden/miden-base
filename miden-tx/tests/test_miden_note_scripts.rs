use miden_lib::{
    assembler,
    notes::{create_note_with_script, Script},
};
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountVault},
    assets::{Asset, FungibleAsset},
    block::BlockHeader,
    chain::ChainMmr,
    notes::{Note, NoteOrigin, NoteScript},
    Felt, StarkField, Word, ONE,
};
use miden_stdlib::StdLibrary;
use mock::{
    constants::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1,
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2, ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
        ACCOUNT_ID_SENDER, DEFAULT_ACCOUNT_CODE,
    },
    mock::{
        account::{mock_account_storage, MockAccountType},
        notes::AssetPreservationStatus,
        transaction::mock_inputs_with_existing,
    },
};

use miden_tx::TransactionExecutor;

mod common;
use common::MockDataStore;

// We test the Pay to ID script. So we create a note that can
// only be consumed by the target account.
#[test]
fn test_p2id_script() {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let target_account = get_account_with_default_account_code(target_account_id, None);

    // Create the note
    let p2id_script = Script::P2ID {
        target: target_account_id,
    };
    let note = create_note_with_script(
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
        MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note.clone()]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(target_account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins = data_store
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    let tx_script = ProgramAst::parse(
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

    // Execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(target_account_id, block_ref, &note_origins, Some(tx_script))
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

    let malicious_account = get_account_with_default_account_code(malicious_account_id, None);

    let data_store_malicious_account =
        MockDataStore::with_existing(Some(malicious_account), Some(vec![note]));
    let mut executor_2 = TransactionExecutor::new(data_store_malicious_account.clone());

    executor_2.load_account(malicious_account_id).unwrap();

    let block_ref = data_store_malicious_account.block_header.block_num().as_int() as u32;
    let note_origins = data_store_malicious_account
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_2 =
        executor_2.execute_transaction(malicious_account_id, block_ref, &note_origins, None);

    // Check that we got the expected result - TransactionExecutorError
    assert!(transaction_result_2.is_err());
}

// We test the Pay to script with 2 assets to test the loop inside the script.
// So we create a note containing two assets that can only be consumed by the target account.
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
    let target_account = get_account_with_default_account_code(target_account_id, None);

    // Create the note
    let p2id_script = Script::P2ID {
        target: target_account_id,
    };
    let note = create_note_with_script(
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
        MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note.clone()]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(target_account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins = data_store
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    let tx_script = ProgramAst::parse(
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

    // Execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(target_account_id, block_ref, &note_origins, Some(tx_script))
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

    let malicious_account = get_account_with_default_account_code(malicious_account_id, None);

    let data_store_malicious_account =
        MockDataStore::with_existing(Some(malicious_account), Some(vec![note]));
    let mut executor_2 = TransactionExecutor::new(data_store_malicious_account.clone());

    executor_2.load_account(malicious_account_id).unwrap();

    let block_ref = data_store_malicious_account.block_header.block_num().as_int() as u32;
    let note_origins = data_store_malicious_account
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_2 =
        executor_2.execute_transaction(malicious_account_id, block_ref, &note_origins, None);

    // Check that we got the expected result - TransactionExecutorError
    assert!(transaction_result_2.is_err());
}

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
    let sender_account = get_account_with_default_account_code(sender_account_id, None);

    // Now create the target account
    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let target_account = get_account_with_default_account_code(target_account_id, None);

    let malicious_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN + 1).unwrap();
    let malicious_account = get_account_with_default_account_code(malicious_account_id, None);

    // --------------------------------------------------------------------------------------------
    // Create notes
    // Create the reclaim block height (Note: Current block height is 4)
    let reclaim_block_height_in_time = 5 as u32;
    let reclaim_block_height_reclaimable = 3 as u32;

    // Create the notes with the P2IDR script
    let p2idr_script_in_time = Script::P2IDR {
        target: target_account_id,
        recall_height: reclaim_block_height_in_time,
    };
    let note_in_time = create_note_with_script(
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
    let note_reclaimable = create_note_with_script(
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
    );
    let mut executor_1 = TransactionExecutor::new(data_store_1.clone());

    executor_1.load_account(target_account_id).unwrap();

    let block_ref_1 = data_store_1.block_header.block_num().as_int() as u32;
    let note_origins = data_store_1
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    let tx_script = ProgramAst::parse(
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

    // Execute the transaction and get the witness
    let transaction_result_1 = executor_1
        .execute_transaction(target_account_id, block_ref_1, &note_origins, Some(tx_script.clone()))
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
    );
    let mut executor_2 = TransactionExecutor::new(data_store_2.clone());

    executor_2.load_account(sender_account_id).unwrap();

    let block_ref_2 = data_store_2.block_header.block_num().as_int() as u32;
    let note_origins_2 = data_store_2
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_2 = executor_2.execute_transaction(
        sender_account_id,
        block_ref_2,
        &note_origins_2,
        Some(tx_script.clone()),
    );

    // Check that we got the expected result - TransactionExecutorError and not TransactionResult
    // Second transaction should not work (sender consumes too early), we expect an error
    assert!(transaction_result_2.is_err());

    // CONSTRUCT AND EXECUTE TX (Case "in time" - Malicious Target Account Failure)
    // --------------------------------------------------------------------------------------------
    let data_store_3 = MockDataStore::with_existing(
        Some(malicious_account.clone()),
        Some(vec![note_in_time.clone()]),
    );
    let mut executor_3 = TransactionExecutor::new(data_store_3.clone());

    executor_3.load_account(malicious_account_id).unwrap();

    let block_ref_3 = data_store_3.block_header.block_num().as_int() as u32;
    let note_origins_3 = data_store_3
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_3 = executor_3.execute_transaction(
        malicious_account_id,
        block_ref_3,
        &note_origins_3,
        Some(tx_script.clone()),
    );

    // Check that we got the expected result - TransactionExecutorError and not TransactionResult
    // Third transaction should not work (malicious account can never consume), we expect an error
    assert!(transaction_result_3.is_err());

    // CONSTRUCT AND EXECUTE TX (Case "reclaimable" - Execution Target Account Success)
    // --------------------------------------------------------------------------------------------
    let data_store_4 = MockDataStore::with_existing(
        Some(target_account.clone()),
        Some(vec![note_reclaimable.clone()]),
    );
    let mut executor_4 = TransactionExecutor::new(data_store_4.clone());

    executor_4.load_account(target_account_id).unwrap();

    let block_ref_4 = data_store_4.block_header.block_num().as_int() as u32;
    let note_origins_4 = data_store_4
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_4 = executor_4
        .execute_transaction(
            target_account_id,
            block_ref_4,
            &note_origins_4,
            Some(tx_script.clone()),
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
    );
    let mut executor_5 = TransactionExecutor::new(data_store_5.clone());

    executor_5.load_account(sender_account_id).unwrap();

    let block_ref_5 = data_store_5.block_header.block_num().as_int() as u32;
    let note_origins = data_store_5
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_5 = executor_5
        .execute_transaction(sender_account_id, block_ref_5, &note_origins, Some(tx_script.clone()))
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
    );
    let mut executor_6 = TransactionExecutor::new(data_store_6.clone());

    executor_6.load_account(malicious_account_id).unwrap();

    let block_ref_6 = data_store_6.block_header.block_num().as_int() as u32;
    let note_origins_6 = data_store_6
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_6 = executor_6.execute_transaction(
        malicious_account_id,
        block_ref_6,
        &note_origins_6,
        Some(tx_script.clone()),
    );

    // Check that we got the expected result - TransactionExecutorError and not TransactionResult
    // Sixth transaction should not work (malicious account can never consume), we expect an error
    assert!(transaction_result_6.is_err())
}

fn get_account_with_default_account_code(account_id: AccountId, assets: Option<Asset>) -> Account {
    let account_code_src = DEFAULT_ACCOUNT_CODE;
    let account_code_ast = ModuleAst::parse(account_code_src).unwrap();
    let mut account_assembler = assembler();

    let account_code = AccountCode::new(account_code_ast.clone(), &mut account_assembler).unwrap();

    let account_storage = mock_account_storage();
    let account_vault = match assets {
        Some(asset) => AccountVault::new(&vec![asset.into()]).unwrap(),
        None => AccountVault::new(&vec![]).unwrap(),
    };

    Account::new(account_id, account_vault, account_storage, account_code, Felt::new(1))
}
