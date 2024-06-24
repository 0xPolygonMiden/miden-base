use miden_objects::{
    accounts::account_id::testing::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
    },
    assets::Asset,
    testing::{
        account::MockAccountType,
        constants::{FUNGIBLE_ASSET_AMOUNT, NON_FUNGIBLE_ASSET_DATA},
        prepare_word,
    },
};

use super::{Felt, Hasher, ProcessState, Word, ONE};
use crate::testing::TransactionContextBuilder;

#[test]
fn test_create_fungible_asset_succeeds() {
    let acc_type = MockAccountType::FungibleFaucet {
        acct_id: ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        nonce: ONE,
        empty_reserved_slot: false,
    };
    let tx_context = TransactionContextBuilder::with_acc_type(acc_type).build();

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::asset

        begin
            # prepare the transaction
            exec.prologue::prepare_transaction

            # push asset amount onto stack
            push.{FUNGIBLE_ASSET_AMOUNT}

            # create fungible asset
            exec.asset::create_fungible_asset
        end
        "
    );

    let process = tx_context.execute_code(&code).unwrap();

    assert_eq!(
        process.get_stack_word(0),
        Word::from([
            Felt::new(FUNGIBLE_ASSET_AMOUNT),
            Felt::new(0),
            Felt::new(0),
            Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN)
        ])
    );
}

#[test]
fn test_create_non_fungible_asset_succeeds() {
    let acc_type = MockAccountType::NonFungibleFaucet {
        acct_id: ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        nonce: ONE,
        empty_reserved_slot: false,
    };
    let tx_context = TransactionContextBuilder::with_acc_type(acc_type).build();

    let non_fungible_asset =
        Asset::mock_non_fungible(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &NON_FUNGIBLE_ASSET_DATA);

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::asset

        begin
            # prepare the transaction
            exec.prologue::prepare_transaction

            # push non-fungible asset data hash onto the stack
            push.{non_fungible_asset_data_hash}
            exec.asset::create_non_fungible_asset
        end
        ",
        non_fungible_asset_data_hash = prepare_word(&Hasher::hash(&NON_FUNGIBLE_ASSET_DATA)),
    );

    let process = tx_context.execute_code(&code).unwrap();

    assert_eq!(process.get_stack_word(0), Word::from(non_fungible_asset));
}

#[test]
fn test_validate_non_fungible_asset() {
    let acc_type = MockAccountType::NonFungibleFaucet {
        acct_id: ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        nonce: ONE,
        empty_reserved_slot: false,
    };
    let tx_context = TransactionContextBuilder::with_acc_type(acc_type).build();

    let non_fungible_asset =
        Asset::mock_non_fungible(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &[1, 2, 3]);
    let encoded = Word::from(non_fungible_asset);

    let code = format!(
        "
        use.miden::kernels::tx::asset

        begin
            push.{asset} exec.asset::validate_non_fungible_asset
        end
        ",
        asset = prepare_word(&encoded)
    );

    let process = tx_context.execute_code(&code).unwrap();
    assert_eq!(process.get_stack_word(0), encoded);
}