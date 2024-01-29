use miden_lib::{accounts::wallets::create_basic_wallet, AuthScheme};
use miden_objects::{
    accounts::{Account, AccountId, AccountStorage, StorageSlotType},
    assembly::ProgramAst,
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::dsa::rpo_falcon512::{KeyPair, PublicKey},
    Felt, Word, ONE, ZERO,
};
use miden_tx::TransactionExecutor;
use mock::{
    constants::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
        ACCOUNT_ID_SENDER,
    },
    procedures::prepare_word,
};

mod common;
use common::{
    get_account_with_default_account_code, get_new_key_pair_with_advice_map,
    get_note_with_fungible_asset_and_script, MockDataStore,
};

#[test]
// Testing the basic Miden wallet - receiving an asset
fn test_receive_asset_via_wallet() {
    // Create assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_1 = FungibleAsset::new(faucet_id_1, 100).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let (target_pub_key, target_keypair_felt) = get_new_key_pair_with_advice_map();
    let target_account =
        get_account_with_default_account_code(target_account_id, target_pub_key, None);

    // Create the note
    let note_script_ast = ProgramAst::parse(
        "
    use.miden::note
    use.miden::contracts::wallets::basic->wallet

    # add the asset
    begin
        dropw
        exec.note::get_assets drop
        mem_loadw
        call.wallet::receive_asset
        dropw
    end
    "
        .to_string()
        .as_str(),
    )
    .unwrap();

    let note = get_note_with_fungible_asset_and_script(fungible_asset_1, note_script_ast);

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store = MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(target_account.id()).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let tx_script_code = ProgramAst::parse(
        "
        use.miden::contracts::auth::basic->auth_tx

        begin
            call.auth_tx::auth_tx_rpo_falcon512
        end
        "
        .to_string()
        .as_str(),
    )
    .unwrap();
    let tx_script = executor
        .compile_tx_script(tx_script_code, vec![(target_pub_key, target_keypair_felt)], vec![])
        .unwrap();

    // Execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(target_account.id(), block_ref, &note_ids, Some(tx_script))
        .unwrap();

    // nonce delta
    assert_eq!(transaction_result.account_delta().nonce(), Some(Felt::new(2)));

    // clone account info
    let account_storage =
        AccountStorage::new(vec![(0, (StorageSlotType::Value { value_arity: 0 }, target_pub_key))])
            .unwrap();
    let account_code = target_account.code().clone();
    // vault delta
    let target_account_after: Account = Account::new(
        target_account.id(),
        AssetVault::new(&[fungible_asset_1.into()]).unwrap(),
        account_storage,
        account_code,
        Felt::new(2),
    );
    assert_eq!(transaction_result.final_account().hash(), target_account_after.hash());
}

#[test]
// Testing the basic Miden wallet - sending an asset
fn test_send_asset_via_wallet() {
    // Mock data
    // We need an asset and an account that owns that asset
    // Create assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();

    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
    let (sender_pub_key, sender_keypair_felt) = get_new_key_pair_with_advice_map();
    let sender_account = get_account_with_default_account_code(
        sender_account_id,
        sender_pub_key,
        fungible_asset_1.into(),
    );

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store = MockDataStore::with_existing(Some(sender_account.clone()), Some(vec![]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(sender_account.id()).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);

    let tx_script_code = ProgramAst::parse(
        format!(
            "
        use.miden::contracts::auth::basic->auth_tx
        use.miden::contracts::wallets::basic->wallet

        begin
            push.{recipient}
            push.{tag}
            push.{asset}
            call.wallet::send_asset drop
            dropw dropw
            call.auth_tx::auth_tx_rpo_falcon512
        end
        ",
            recipient = prepare_word(&recipient),
            tag = tag,
            asset = prepare_word(&fungible_asset_1.into())
        )
        .as_str(),
    )
    .unwrap();
    let tx_script = executor
        .compile_tx_script(tx_script_code, vec![(sender_pub_key, sender_keypair_felt)], vec![])
        .unwrap();

    // Execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(sender_account.id(), block_ref, &note_ids, Some(tx_script))
        .unwrap();

    // clones account info
    let sender_account_storage =
        AccountStorage::new(vec![(0, (StorageSlotType::Value { value_arity: 0 }, sender_pub_key))])
            .unwrap();
    let sender_account_code = sender_account.code().clone();

    // vault delta
    let sender_account_after: Account = Account::new(
        data_store.account.id(),
        AssetVault::new(&[]).unwrap(),
        sender_account_storage,
        sender_account_code,
        Felt::new(2),
    );
    assert_eq!(transaction_result.final_account().hash(), sender_account_after.hash());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_wallet_creation() {
    // we need a Falcon Public Key to create the wallet account

    use miden_objects::accounts::AccountType;
    let key_pair: KeyPair = KeyPair::new().unwrap();
    let pub_key: PublicKey = key_pair.public_key();
    let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key };

    // we need to use an initial seed to create the wallet account
    let init_seed: [u8; 32] = [
        95, 113, 209, 94, 84, 105, 250, 242, 223, 203, 216, 124, 22, 159, 14, 132, 215, 85, 183,
        204, 149, 90, 166, 68, 100, 73, 106, 168, 125, 237, 138, 16,
    ];

    let (wallet, _) =
        create_basic_wallet(init_seed, auth_scheme, AccountType::RegularAccountImmutableCode)
            .unwrap();

    // sender_account_id not relevant here, just to create a default account code
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
    let expected_code_root =
        get_account_with_default_account_code(sender_account_id, pub_key.into(), None)
            .code()
            .root();

    assert!(wallet.is_regular_account());
    assert_eq!(wallet.code().root(), expected_code_root);
    let pub_key_word: Word = pub_key.into();
    assert_eq!(wallet.storage().get_item(0).as_elements(), pub_key_word);
}
