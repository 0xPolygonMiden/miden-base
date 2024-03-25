use miden_objects::{
    accounts::{Account, AccountId, AccountCode, AccountStorage, SlotItem, StorageSlot},
    assembly::{ProgramAst, ModuleAst},
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::rand::RpoRandomCoin,
    transaction::TransactionArgs,
    Felt, Word,
    notes::{Note, NoteScript},
    NoteError, 
    crypto::rand::FeltRng,};
use miden_tx::TransactionExecutor;
use miden_lib::transaction::TransactionKernel;
use mock::mock::account::{
    ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
    ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN, ACCOUNT_ID_SENDER, DEFAULT_AUTH_SCRIPT,
};

use crate::{
    get_new_key_pair_with_advice_map, MockDataStore,
};

//use crate::prove_and_verify_transaction;

pub fn get_account_with_custom_account_code(
    account_id: AccountId,
    public_key: Word,
    assets: Option<Asset>,
) -> Account {
    let account_code_src = include_str!("Poker_Account.masm");
    let account_code_ast = ModuleAst::parse(account_code_src).unwrap();
    let account_assembler = TransactionKernel::assembler();

    let account_code = AccountCode::new(account_code_ast.clone(), &account_assembler).unwrap();
    let account_storage = AccountStorage::new(vec![SlotItem {
        index: 0,
        slot: StorageSlot::new_value(public_key),
    }])
    .unwrap();

    let account_vault = match assets {
        Some(asset) => AssetVault::new(&[asset]).unwrap(),
        None => AssetVault::new(&[]).unwrap(),
    };

    Account::new(account_id, account_vault, account_storage, account_code, Felt::new(1))
}

fn create_note<R: FeltRng>(
    sender_account_id: AccountId,
    target_account_id: AccountId,
    assets: Vec<Asset>,
    mut rng: R,
) -> Result<Note, NoteError> {
    let note_script = include_str!("Note_Script.masm");
    
    let note_assembler = TransactionKernel::assembler();
    let script_ast = ProgramAst::parse(note_script).unwrap();
    let (note_script, _) = NoteScript::new(script_ast, &note_assembler)?;

    let card_1 = [Felt::new(99), Felt::new(99), Felt::new(99), Felt::new(99)];
    let card_2 = [Felt::new(98), Felt::new(98), Felt::new(98), Felt::new(98)];
    
    // Here you can add the inputs to the note
    let inputs = [card_1.as_slice(), card_2.as_slice()].concat();
    let tag: Felt = target_account_id.into();
    let serial_num = rng.draw_word();

    Note::new(note_script, &inputs, &assets, serial_num, sender_account_id, tag)
}

// Note TESTS
// ===============================================================================================
// We test the Note script.
#[test]
fn note_script_poker() {
    // Create an asset
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let (target_pub_key, target_sk_pk_felt) = get_new_key_pair_with_advice_map();
    let target_account =
    get_account_with_custom_account_code(target_account_id, target_pub_key, None);

    // Create the note
    let note = create_note(
        sender_account_id,
        target_account_id,
        vec![fungible_asset],
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store =
        MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note.clone()]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(target_account_id).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let tx_script_code = ProgramAst::parse(DEFAULT_AUTH_SCRIPT).unwrap();

    let tx_script_target = executor
        .compile_tx_script(
            tx_script_code.clone(),
            vec![(target_pub_key, target_sk_pk_felt)],
            vec![],
        )
        .unwrap();
    let tx_args_target = TransactionArgs::new(Some(tx_script_target), None);

    // Execute the transaction and get the witness
    let executed_transaction = executor
        .execute_transaction(target_account_id, block_ref, &note_ids, Some(tx_args_target))
        .unwrap();

    // Prove, serialize/deserialize and verify the transaction
    // We can add this as a last step
    //assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    // Not sure what you want to test after the account but we should see if the 
    // account change is what you expect
    let mut target_storage = target_account.storage().clone();
    target_storage.set_item(100, [Felt::new(99), Felt::new(99), Felt::new(99), Felt::new(99)]).unwrap();
    target_storage.set_item(101, [Felt::new(98), Felt::new(98), Felt::new(98), Felt::new(98)]).unwrap();
    
    let target_account_after: Account = Account::new(
        target_account.id(),
        AssetVault::new(&[fungible_asset]).unwrap(),
        target_storage,
        target_account.code().clone(),
        Felt::new(2),
    );
    assert_eq!(executed_transaction.final_account().hash(), target_account_after.hash());
}
