use common::{
    get_account_with_default_account_code, get_new_key_pair_with_advice_map, MockDataStore,
};
use miden_lib::notes::{create_note, Script};
use miden_objects::{
    accounts::{Account, AccountId, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN},
    assets::{Asset, FungibleAsset},
    Felt,
};
use miden_tx::TransactionExecutor;
use mock::constants::{ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN, ACCOUNT_ID_SENDER};

mod common;

#[test]
fn test_atomic_swap_script() {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    // Create sender and target accounts
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let (target_pub_key, target_sk_pk_felt) = get_new_key_pair_with_advice_map();
    let target_account =
        get_account_with_default_account_code(target_account_id, target_pub_key, None);

    // Create the note
    let aswap_script = Script::ASWAP {};
    let note = create_note(
        aswap_script,
        vec![fungible_asset],
        sender_account_id,
        None,
        [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)],
    )
    .unwrap();

    // Construct and execute tx
    let data_store =
        MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note.clone()]), None);

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(target_account_id).unwrap();
}
