use alloc::sync::Arc;

use miden_lib::{accounts::wallets::create_basic_wallet, AuthScheme};
use miden_objects::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_OFF_CHAIN_SENDER,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        },
        Account, AccountId, AccountStorage, StorageMap, StorageSlot,
    },
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::dsa::rpo_falcon512::SecretKey,
    notes::{NoteExecutionHint, NoteTag, NoteType},
    testing::prepare_word,
    transaction::TransactionArgs,
    Felt, Word, ONE, ZERO,
};
use miden_tx::{testing::TransactionContextBuilder, TransactionExecutor};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};

use crate::{
    build_default_auth_script, build_tx_args_from_script,
    get_account_with_basic_authenticated_wallet, get_new_pk_and_authenticator,
    get_note_with_fungible_asset_and_script, prove_and_verify_transaction,
};

// Testing the basic Miden wallet - receiving an asset
#[test]
fn prove_receive_asset_via_wallet() {
    // Create assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_1 = FungibleAsset::new(faucet_id_1, 100).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN).unwrap();
    let (target_pub_key, target_falcon_auth) = get_new_pk_and_authenticator();
    let target_account =
        get_account_with_basic_authenticated_wallet(target_account_id, target_pub_key, None);

    // Create the note
    let note_script_src = "
    # add the asset
    begin
        dropw
        exec.::miden::note::get_assets drop
        mem_loadw
        call.::miden::contracts::wallets::basic::receive_asset
        dropw
    end
    ";

    let note = get_note_with_fungible_asset_and_script(fungible_asset_1, note_script_src);

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let tx_context = TransactionContextBuilder::new(target_account.clone())
        .input_notes(vec![note])
        .build();

    let executor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), Some(target_falcon_auth.clone()));

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let tx_script = build_default_auth_script();
    let tx_args = TransactionArgs::with_tx_script(tx_script);

    // Execute the transaction and get the witness
    let executed_transaction = executor
        .execute_transaction(target_account.id(), block_ref, &note_ids, tx_args)
        .unwrap();

    // Prove, serialize/deserialize and verify the transaction
    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    // nonce delta
    assert_eq!(executed_transaction.account_delta().nonce(), Some(Felt::new(2)));

    // clone account info
    let account_storage = AccountStorage::new(vec![
        StorageSlot::Value(target_pub_key),
        StorageSlot::Value(Word::default()),
        StorageSlot::Map(StorageMap::default()),
    ])
    .unwrap();
    let account_code = target_account.code().clone();
    // vault delta
    let target_account_after: Account = Account::from_parts(
        target_account.id(),
        AssetVault::new(&[fungible_asset_1.into()]).unwrap(),
        account_storage,
        account_code,
        Felt::new(2),
    );
    assert_eq!(executed_transaction.final_account().hash(), target_account_after.hash());
}

/// Testing sending a note without assets from the basic wallet
#[test]
fn prove_send_note_without_asset_via_wallet() {
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_OFF_CHAIN_SENDER).unwrap();
    let (sender_pub_key, sender_falcon_auth) = get_new_pk_and_authenticator();
    let sender_account =
        get_account_with_basic_authenticated_wallet(sender_account_id, sender_pub_key, None);

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let tx_context = TransactionContextBuilder::new(sender_account.clone()).build();

    let executor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), Some(sender_falcon_auth.clone()));

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let aux = Felt::new(27);
    let tag = NoteTag::for_local_use_case(0, 0).unwrap();
    let note_type = NoteType::Private;

    assert_eq!(tag.validate(note_type), Ok(tag));

    let tx_script_src = &format!(
        "
        begin
            padw padw
            push.{recipient}
            push.{note_execution_hint}
            push.{note_type}
            push.{aux}
            push.{tag}
            call.::miden::contracts::wallets::basic::create_note
            dropw dropw dropw dropw
        end
        ",
        recipient = prepare_word(&recipient),
        note_type = note_type as u8,
        tag = tag,
        note_execution_hint = Felt::from(NoteExecutionHint::always())
    );
    let tx_args = build_tx_args_from_script(tx_script_src);

    let executed_transaction = executor
        .execute_transaction(sender_account.id(), block_ref, &note_ids, tx_args)
        .unwrap();

    prove_and_verify_transaction(executed_transaction.clone()).unwrap();

    // clones account info
    let sender_account_storage = AccountStorage::new(vec![
        StorageSlot::Value(sender_pub_key),
        StorageSlot::Value(Word::default()),
        StorageSlot::Map(StorageMap::default()),
    ])
    .unwrap();
    let sender_account_code = sender_account.code().clone();

    // vault delta
    let sender_account_after: Account = Account::from_parts(
        tx_context.account().id(),
        AssetVault::new(&[]).unwrap(),
        sender_account_storage,
        sender_account_code,
        // state of the account did not change, so nonce should remain the same
        Felt::new(1),
    );

    assert_eq!(executed_transaction.final_account().hash(), sender_account_after.hash());
}

/// Testing the basic Miden wallet - creating a note and moving asset to it
#[test]
fn prove_send_note_with_asset_via_wallet() {
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();

    let sender_account_id = AccountId::try_from(ACCOUNT_ID_OFF_CHAIN_SENDER).unwrap();
    let (sender_pub_key, sender_falcon_auth) = get_new_pk_and_authenticator();
    let sender_account = get_account_with_basic_authenticated_wallet(
        sender_account_id,
        sender_pub_key,
        fungible_asset_1.into(),
    );

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let tx_context = TransactionContextBuilder::new(sender_account.clone()).build();

    let executor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), Some(sender_falcon_auth.clone()));

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let aux = Felt::new(27);
    let tag = NoteTag::for_local_use_case(0, 0).unwrap();
    let note_type = NoteType::Private;

    assert_eq!(tag.validate(note_type), Ok(tag));

    let tx_script_src = &format!(
        "
        begin
            padw padw
            push.{recipient}
            push.{note_execution_hint}
            push.{note_type}
            push.{aux}
            push.{tag}
            call.::miden::contracts::wallets::basic::create_note
            # => [note_idx, PAD(15)]

            swapw dropw 
            push.{asset}
            call.::miden::contracts::wallets::basic::move_asset_to_note
            dropw dropw dropw dropw
            
            call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
        end
        ",
        recipient = prepare_word(&recipient),
        note_type = note_type as u8,
        tag = tag,
        asset = prepare_word(&fungible_asset_1.into()),
        note_execution_hint = Felt::from(NoteExecutionHint::always())
    );
    let tx_args = build_tx_args_from_script(tx_script_src);

    let executed_transaction = executor
        .execute_transaction(sender_account.id(), block_ref, &note_ids, tx_args)
        .unwrap();

    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    // clones account info
    let sender_account_storage = AccountStorage::new(vec![
        StorageSlot::Value(sender_pub_key),
        StorageSlot::Value(Word::default()),
        StorageSlot::Map(StorageMap::default()),
    ])
    .unwrap();
    let sender_account_code = sender_account.code().clone();

    // vault delta
    let sender_account_after: Account = Account::from_parts(
        tx_context.account().id(),
        AssetVault::new(&[]).unwrap(),
        sender_account_storage,
        sender_account_code,
        Felt::new(2),
    );
    assert_eq!(executed_transaction.final_account().hash(), sender_account_after.hash());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn wallet_creation() {
    use miden_lib::accounts::{auth::RpoFalcon512, wallets::BasicWallet};
    use miden_objects::accounts::{AccountCode, AccountStorageMode, AccountType};

    // we need a Falcon Public Key to create the wallet account
    let seed = [0_u8; 32];
    let mut rng = ChaCha20Rng::from_seed(seed);

    let sec_key = SecretKey::with_rng(&mut rng);
    let pub_key = sec_key.public_key();
    let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key };

    // we need to use an initial seed to create the wallet account
    let init_seed: [u8; 32] = [
        95, 113, 209, 94, 84, 105, 250, 242, 223, 203, 216, 124, 22, 159, 14, 132, 215, 85, 183,
        204, 149, 90, 166, 68, 100, 73, 106, 168, 125, 237, 138, 16,
    ];

    let account_type = AccountType::RegularAccountImmutableCode;
    let storage_mode = AccountStorageMode::Private;

    let (wallet, _) =
        create_basic_wallet(init_seed, auth_scheme, account_type, storage_mode).unwrap();

    let expected_code = AccountCode::from_components(
        &[RpoFalcon512::new(pub_key).into(), BasicWallet.into()],
        AccountType::RegularAccountUpdatableCode,
    )
    .unwrap();
    let expected_code_commitment = expected_code.commitment();

    assert!(wallet.is_regular_account());
    assert_eq!(wallet.code().commitment(), expected_code_commitment);
    let pub_key_word: Word = pub_key.into();
    assert_eq!(wallet.storage().get_item(0).unwrap().as_elements(), pub_key_word);
}
