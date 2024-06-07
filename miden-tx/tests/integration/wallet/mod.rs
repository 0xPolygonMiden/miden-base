use miden_lib::{accounts::wallets::create_basic_wallet, AuthScheme};
use miden_objects::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_OFF_CHAIN_SENDER,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        },
        Account, AccountId, AccountStorage, SlotItem,
    },
    assembly::ProgramAst,
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::dsa::rpo_falcon512::SecretKey,
    notes::{NoteTag, NoteType},
    testing::{account_code::DEFAULT_AUTH_SCRIPT, prepare_word},
    transaction::TransactionArgs,
    Felt, Word, ONE, ZERO,
};
use miden_tx::{testing::data_store::MockDataStore, TransactionExecutor};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};

use crate::{
    get_account_with_default_account_code, get_new_pk_and_authenticator,
    get_note_with_fungible_asset_and_script, prove_and_verify_transaction,
};

#[test]
// Testing the basic Miden wallet - receiving an asset
fn prove_receive_asset_via_wallet() {
    // Create assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_1 = FungibleAsset::new(faucet_id_1, 100).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN).unwrap();
    let (target_pub_key, target_falcon_auth) = get_new_pk_and_authenticator();
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

    let mut executor =
        TransactionExecutor::new(data_store.clone(), Some(target_falcon_auth.clone()));
    executor.load_account(target_account.id()).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let tx_script_code = ProgramAst::parse(DEFAULT_AUTH_SCRIPT).unwrap();
    let tx_script = executor.compile_tx_script(tx_script_code, vec![], vec![]).unwrap();
    let tx_args: TransactionArgs = TransactionArgs::with_tx_script(tx_script);

    // Execute the transaction and get the witness
    let executed_transaction = executor
        .execute_transaction(target_account.id(), block_ref, &note_ids, tx_args)
        .unwrap();

    // Prove, serialize/deserialize and verify the transaction
    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    // nonce delta
    assert_eq!(executed_transaction.account_delta().nonce(), Some(Felt::new(2)));

    // clone account info
    let account_storage =
        AccountStorage::new(vec![SlotItem::new_value(0, 0, target_pub_key)], vec![]).unwrap();
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

#[test]
/// Testing the basic Miden wallet - sending an asset
fn prove_send_asset_via_wallet() {
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();

    let sender_account_id = AccountId::try_from(ACCOUNT_ID_OFF_CHAIN_SENDER).unwrap();
    let (sender_pub_key, sender_falcon_auth) = get_new_pk_and_authenticator();
    let sender_account = get_account_with_default_account_code(
        sender_account_id,
        sender_pub_key,
        fungible_asset_1.into(),
    );

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store = MockDataStore::with_existing(Some(sender_account.clone()), Some(vec![]));

    let mut executor =
        TransactionExecutor::new(data_store.clone(), Some(sender_falcon_auth.clone()));
    executor.load_account(sender_account.id()).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = NoteTag::for_local_use_case(0, 0).unwrap();
    let note_type = NoteType::OffChain;

    assert_eq!(tag.validate(note_type), Ok(tag));

    let var_name = &format!(
        "
        use.miden::contracts::auth::basic->auth_tx
        use.miden::contracts::wallets::basic->wallet

        begin
            push.{recipient}
            push.{note_type}
            push.{tag}
            push.{asset}
            call.wallet::send_asset
            drop drop dropw dropw
            call.auth_tx::auth_tx_rpo_falcon512
        end
        ",
        recipient = prepare_word(&recipient),
        note_type = note_type as u8,
        tag = tag,
        asset = prepare_word(&fungible_asset_1.into())
    );
    let tx_script_code = ProgramAst::parse(var_name.as_str()).unwrap();
    let tx_script = executor.compile_tx_script(tx_script_code, vec![], vec![]).unwrap();
    let tx_args: TransactionArgs = TransactionArgs::with_tx_script(tx_script);

    let executed_transaction = executor
        .execute_transaction(sender_account.id(), block_ref, &note_ids, tx_args)
        .unwrap();

    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    // clones account info
    let sender_account_storage =
        AccountStorage::new(vec![SlotItem::new_value(0, 0, sender_pub_key)], vec![]).unwrap();
    let sender_account_code = sender_account.code().clone();

    // vault delta
    let sender_account_after: Account = Account::from_parts(
        data_store.account.id(),
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
    use miden_objects::accounts::{
        account_id::testing::ACCOUNT_ID_SENDER, AccountStorageType, AccountType,
    };

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
    let storage_type = AccountStorageType::OffChain;

    let (wallet, _) =
        create_basic_wallet(init_seed, auth_scheme, account_type, storage_type).unwrap();

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
