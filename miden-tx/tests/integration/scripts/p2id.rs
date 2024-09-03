use std::rc::Rc;

use miden_lib::{notes::create_p2id_note, transaction::TransactionKernel};
use miden_objects::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2, ACCOUNT_ID_SENDER,
        },
        Account, AccountId, AccountType, StorageSlot,
    },
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::rand::RpoRandomCoin,
    notes::NoteType,
    testing::{account::AccountBuilder, account_code::DEFAULT_AUTH_SCRIPT},
    transaction::{TransactionArgs, TransactionScript},
    Felt, FieldElement,
};
use miden_tx::{
    auth::BasicAuthenticator,
    testing::{
        mock_chain::{Auth, MockChain},
        TransactionContextBuilder,
    },
    TransactionExecutor,
};
use rand::{rngs::StdRng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use vm_processor::Word;

use crate::{
    build_default_auth_script, get_account_with_default_account_code, get_new_pk_and_authenticator,
    prove_and_verify_transaction,
};

// P2ID TESTS
// ===============================================================================================
// We test the Pay to ID script. So we create a note that can only be consumed by the target
// account.
#[test]
fn prove_p2id_script() {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN).unwrap();
    let (target_pub_key, falcon_auth) = get_new_pk_and_authenticator();

    let target_account =
        get_account_with_default_account_code(target_account_id, target_pub_key, None);

    // Create the note
    let note = create_p2id_note(
        sender_account_id,
        target_account_id,
        vec![fungible_asset],
        NoteType::Public,
        Felt::new(0),
        &mut RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let tx_context = TransactionContextBuilder::new(target_account.clone())
        .input_notes(vec![note.clone()])
        .build();

    let executor = TransactionExecutor::new(tx_context.clone(), Some(falcon_auth.clone()));

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let tx_script_target = build_default_auth_script();
    let tx_args_target = TransactionArgs::with_tx_script(tx_script_target);

    // Execute the transaction and get the witness
    let executed_transaction = executor
        .execute_transaction(target_account_id, block_ref, &note_ids, tx_args_target)
        .unwrap();

    // Prove, serialize/deserialize and verify the transaction
    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    // vault delta
    let target_account_after: Account = Account::from_parts(
        target_account.id(),
        AssetVault::new(&[fungible_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );
    assert_eq!(executed_transaction.final_account().hash(), target_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Failure)
    // --------------------------------------------------------------------------------------------
    // A "malicious" account tries to consume the note, we expect an error

    let malicious_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2).unwrap();
    let (malicious_pub_key, malicious_falcon_auth) = get_new_pk_and_authenticator();
    let malicious_account =
        get_account_with_default_account_code(malicious_account_id, malicious_pub_key, None);

    let tx_context_malicious_account = TransactionContextBuilder::new(malicious_account)
        .input_notes(vec![note])
        .build();
    let executor_2 =
        TransactionExecutor::new(tx_context_malicious_account.clone(), Some(malicious_falcon_auth));

    let tx_script_malicious = build_default_auth_script();
    let tx_args_malicious = TransactionArgs::with_tx_script(tx_script_malicious);

    let block_ref = tx_context_malicious_account.tx_inputs().block_header().block_num();
    let note_ids = tx_context_malicious_account
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let executed_transaction_2 = executor_2.execute_transaction(
        malicious_account_id,
        block_ref,
        &note_ids,
        tx_args_malicious,
    );

    // Check that we got the expected result - TransactionExecutorError
    assert!(executed_transaction_2.is_err());
}

/// We test the Pay to script with 2 assets to test the loop inside the script.
/// So we create a note containing two assets that can only be consumed by the target account.
#[test]
fn p2id_script_multiple_assets() {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id, 123).unwrap().into();

    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap();
    let fungible_asset_2: Asset = FungibleAsset::new(faucet_id_2, 456).unwrap().into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();

    let (target_pub_key, falcon_auth) = get_new_pk_and_authenticator();
    let target_account =
        get_account_with_default_account_code(target_account_id, target_pub_key, None);

    // Create the note
    let note = create_p2id_note(
        sender_account_id,
        target_account_id,
        vec![fungible_asset_1, fungible_asset_2],
        NoteType::Public,
        Felt::new(0),
        &mut RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let tx_context = TransactionContextBuilder::new(target_account.clone())
        .input_notes(vec![note.clone()])
        .build();

    let executor = TransactionExecutor::new(tx_context.clone(), Some(falcon_auth));

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let tx_script_target = build_default_auth_script();
    let tx_args_target = TransactionArgs::with_tx_script(tx_script_target);

    // Execute the transaction and get the witness
    let executed_transaction = executor
        .execute_transaction(target_account_id, block_ref, &note_ids, tx_args_target)
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
    // A "malicious" account tries to consume the note, we expect an error

    let malicious_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2).unwrap();
    let (malicious_pub_key, malicious_falcon_auth) = get_new_pk_and_authenticator();
    let malicious_account =
        get_account_with_default_account_code(malicious_account_id, malicious_pub_key, None);

    let tx_context_malicious_account = TransactionContextBuilder::new(malicious_account)
        .input_notes(vec![note])
        .build();
    let executor_2 =
        TransactionExecutor::new(tx_context_malicious_account.clone(), Some(malicious_falcon_auth));
    let tx_script_malicious = build_default_auth_script();
    let tx_args_malicious = TransactionArgs::with_tx_script(tx_script_malicious);

    let block_ref = tx_context_malicious_account.tx_inputs().block_header().block_num();
    let note_origins = tx_context_malicious_account
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let executed_transaction_2 = executor_2.execute_transaction(
        malicious_account_id,
        block_ref,
        &note_origins,
        tx_args_malicious,
    );

    // Check that we got the expected result - TransactionExecutorError
    assert!(executed_transaction_2.is_err());
}

/// Consumes an existing note with a new account
#[test]
fn prove_consume_note_with_new_account() {
    let (mut target_account, seed, falcon_auth) = create_new_account();
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id, 123).unwrap().into();

    // Create the note
    let note = create_p2id_note(
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap(),
        target_account.id(),
        vec![fungible_asset_1],
        NoteType::Public,
        Felt::new(0),
        &mut RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    let tx_context = TransactionContextBuilder::new(target_account.clone())
        .account_seed(Some(seed))
        .input_notes(vec![note.clone()])
        .build();

    assert!(target_account.is_new());

    let executor = TransactionExecutor::new(tx_context.clone(), Some(falcon_auth));

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let tx_script_target = build_default_auth_script();
    let tx_args_target = TransactionArgs::with_tx_script(tx_script_target);

    // Execute the transaction and get the witness
    let executed_transaction = executor
        .execute_transaction(target_account.id(), block_ref, &note_ids, tx_args_target)
        .unwrap();

    // Account delta
    target_account.apply_delta(executed_transaction.account_delta()).unwrap();
    assert!(!target_account.is_new());

    assert!(prove_and_verify_transaction(executed_transaction).is_ok());
}

/// Consumes two existing notes (with an asset from a faucet for a combined total of 123 tokens)
/// with a basic account
#[test]
fn prove_consume_multiple_notes() {
    let mut mock_chain = MockChain::new();
    let mut account = mock_chain.add_existing_wallet(Auth::BasicAuth, vec![]);

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
        )
        .unwrap();
    let note_2 = mock_chain
        .add_p2id_note(
            ACCOUNT_ID_SENDER.try_into().unwrap(),
            account.id(),
            &[fungible_asset_2],
            NoteType::Private,
        )
        .unwrap();

    let tx_script =
        TransactionScript::compile(DEFAULT_AUTH_SCRIPT, vec![], TransactionKernel::assembler())
            .unwrap();
    let tx_context = mock_chain
        .build_tx_context(account.id())
        .input_notes(vec![note_1, note_2])
        .tx_script(tx_script)
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

// HELPER FUNCTIONS
// ===============================================================================================

fn create_new_account() -> (Account, Word, Rc<BasicAuthenticator<StdRng>>) {
    let (pub_key, falcon_auth) = get_new_pk_and_authenticator();

    let storage_slot = StorageSlot::Value(pub_key);

    let (account, seed) = AccountBuilder::new(ChaCha20Rng::from_entropy())
        .add_storage_slot(storage_slot)
        .default_code(TransactionKernel::assembler_testing())
        .account_type(AccountType::RegularAccountUpdatableCode)
        .nonce(Felt::ZERO)
        .build()
        .unwrap();

    (account, seed, falcon_auth)
}
