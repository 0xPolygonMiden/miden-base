use common::{
    get_account_with_default_account_code, get_new_key_pair_with_advice_map, MockDataStore,
};
use miden_lib::notes::{create_note, Script};
use miden_objects::{
    accounts::{Account, AccountId, AccountVault, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN},
    assembly::ProgramAst,
    assets::{Asset, FungibleAsset},
    Felt,
};
use miden_tx::TransactionExecutor;
use mock::constants::{ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN, ACCOUNT_ID_SENDER};
use vm_core::StarkField;

mod common;

#[test]
fn test_atomic_swap_script() {
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
    let aswap_script = Script::ASWAP {
        faucet_id,
        amount: Felt::new(100),
        tag: Felt::new(0),
        // TODO: Create Digest
        recipient: Felt::new(99),
    };
    let note = create_note(
        aswap_script,
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
    use.miden::auth::basic->auth_tx

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
}
