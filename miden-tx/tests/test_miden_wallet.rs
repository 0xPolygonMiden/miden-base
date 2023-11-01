use miden_lib::{assembler::assembler, wallets::create_basic_wallet, AuthScheme};
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountVault},
    assembly::{ModuleAst, ProgramAst},
    assets::{Asset, FungibleAsset},
    crypto::dsa::rpo_falcon512,
    notes::{Note, NoteScript},
    Felt, StarkField, Word, ONE, ZERO,
};
use mock::{
    constants::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
        ACCOUNT_ID_SENDER, DEFAULT_ACCOUNT_CODE,
    },
    mock::account::mock_account_storage,
    utils::prepare_word,
};

use miden_tx::TransactionExecutor;

mod common;
use common::MockDataStore;

use vm_core::crypto::dsa::rpo_falcon512::{KeyPair, PublicKey};

#[test]
// Testing the basic Miden wallet - receiving an asset
fn test_receive_asset_via_wallet() {
    // Create assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_1 = FungibleAsset::new(faucet_id_1, 100).unwrap();

    let target_account = get_account_with_default_account_code(None);

    // Create the note
    let note_script_ast = ProgramAst::parse(
        format!(
            "
    use.miden::sat::note
    use.miden::wallets::basic->wallet

    # add the asset
    begin
        dropw
        exec.note::get_assets drop
        mem_loadw
        call.wallet::receive_asset
        dropw
    end
    "
        )
        .as_str(),
    )
    .unwrap();

    let note = get_note_with_fungible_asset_and_script(fungible_asset_1.clone(), note_script_ast);

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store = MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(target_account.id()).unwrap();

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
    let tx_script = executor.compile_tx_script(tx_script_code, vec![], vec![]).unwrap();

    // Execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(target_account.id(), block_ref, &note_origins, Some(tx_script))
        .unwrap();

    // nonce delta
    assert!(transaction_result.account_delta().nonce == Some(Felt::new(2)));

    // clone account info
    let account_storage = mock_account_storage();
    let account_code = target_account.code().clone();
    // vault delta
    let target_account_after: Account = Account::new(
        target_account.id(),
        AccountVault::new(&vec![fungible_asset_1.into()]).unwrap(),
        account_storage,
        account_code,
        Felt::new(2),
    );
    assert!(transaction_result.final_account_hash() == target_account_after.hash());
}

#[test]
// Testing the basic Miden wallet - sending an asset
fn test_send_asset_via_wallet() {
    // Mock data
    // We need an asset and an account that owns that asset
    // Create assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();

    let sender_account = get_account_with_default_account_code(fungible_asset_1.clone().into());

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store = MockDataStore::with_existing(Some(sender_account.clone()), Some(vec![]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(sender_account.id()).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins =
        data_store.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);

    let tx_script_code = ProgramAst::parse(
        format!(
            "
        use.miden::eoa::basic->auth_tx
        use.miden::wallets::basic->wallet

        begin
            push.{recipient}
            push.{tag}
            push.{asset}
            call.wallet::send_asset drop
            call.auth_tx::auth_tx_rpo_falcon512
            dropw dropw
        end
        ",
            recipient = prepare_word(&recipient),
            tag = tag,
            asset = prepare_word(&fungible_asset_1.try_into().unwrap())
        )
        .as_str(),
    )
    .unwrap();
    let tx_script = executor.compile_tx_script(tx_script_code, vec![], vec![]).unwrap();

    // Execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(sender_account.id(), block_ref, &note_origins, Some(tx_script))
        .unwrap();

    // clones account info
    let sender_account_storage = mock_account_storage();
    let sender_account_code = sender_account.code().clone();

    // vault delta
    let sender_account_after: Account = Account::new(
        sender_account.id(),
        AccountVault::new(&vec![]).unwrap(),
        sender_account_storage,
        sender_account_code,
        Felt::new(2),
    );
    assert!(transaction_result.final_account_hash() == sender_account_after.hash());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_wallet_creation() {
    // we need a Falcon Public Key to create the wallet account
    let key_pair: KeyPair = rpo_falcon512::KeyPair::new().unwrap();
    let pub_key: PublicKey = key_pair.public_key();
    let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 {
        pub_key: pub_key.into(),
    };

    // we need to use an initial seed to create the wallet account
    let init_seed: [u8; 32] = [
        95, 113, 209, 94, 84, 105, 250, 242, 223, 203, 216, 124, 22, 159, 14, 132, 215, 85, 183,
        204, 149, 90, 166, 68, 100, 73, 106, 168, 125, 237, 138, 16,
    ];

    let (wallet, _) = create_basic_wallet(init_seed, auth_scheme).unwrap();

    let expected_code_root = get_account_with_default_account_code(None).code().root();

    assert!(wallet.is_regular_account() == true);
    assert!(wallet.code().root() == expected_code_root);
    let pub_key_word: Word = pub_key.into();
    assert!(wallet.storage().get_item(0).as_elements() == pub_key_word);
}

fn get_account_with_default_account_code(asset: Option<Asset>) -> Account {
    let account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let account_code_src = DEFAULT_ACCOUNT_CODE;
    let account_code_ast = ModuleAst::parse(account_code_src).unwrap();
    let mut account_assembler = assembler();

    let account_code = AccountCode::new(account_code_ast.clone(), &mut account_assembler).unwrap();

    let account_storage = mock_account_storage();
    let account_vault = match asset {
        Some(asset) => AccountVault::new(&vec![asset.into()]).unwrap(),
        None => AccountVault::new(&vec![]).unwrap(),
    };

    Account::new(account_id, account_vault, account_storage, account_code, Felt::new(1))
}

fn get_note_with_fungible_asset_and_script(
    fungible_asset: FungibleAsset,
    note_script: ProgramAst,
) -> Note {
    let mut note_assembler = assembler();

    let (note_script, _) = NoteScript::new(note_script, &mut note_assembler).unwrap();
    const SERIAL_NUM: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let sender_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    Note::new(
        note_script.clone(),
        &[],
        &vec![fungible_asset.into()],
        SERIAL_NUM,
        sender_id,
        Felt::new(1),
    )
    .unwrap()
}
