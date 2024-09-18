use miden_objects::{
    accounts::account_id::testing::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
    },
    assets::NonFungibleAsset,
    testing::{
        constants::{
            FUNGIBLE_ASSET_AMOUNT, FUNGIBLE_FAUCET_INITIAL_BALANCE, NON_FUNGIBLE_ASSET_DATA,
        },
        prepare_word,
    },
};
use vm_processor::ProcessState;

use super::{Felt, Hasher, Word, ONE};
use crate::testing::TransactionContextBuilder;

#[test]
fn test_create_fungible_asset_succeeds() {
    let tx_context = TransactionContextBuilder::with_fungible_faucet(
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ONE,
        Felt::new(FUNGIBLE_FAUCET_INITIAL_BALANCE),
    )
    .build();

    let code = format!(
        "
        use.kernel::prologue
        use.miden::asset

        begin
            exec.prologue::prepare_transaction

            # create fungible asset
            push.{FUNGIBLE_ASSET_AMOUNT}
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
    let tx_context = TransactionContextBuilder::with_non_fungible_faucet(
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        ONE,
        false,
    )
    .build();

    let non_fungible_asset =
        NonFungibleAsset::mock(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &NON_FUNGIBLE_ASSET_DATA);

    let code = format!(
        "
        use.kernel::prologue
        use.miden::asset

        begin
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
    let tx_context = TransactionContextBuilder::with_non_fungible_faucet(
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        ONE,
        false,
    )
    .build();

    let non_fungible_asset =
        NonFungibleAsset::mock(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &[1, 2, 3]);
    let encoded = Word::from(non_fungible_asset);

    let code = format!(
        "
        use.kernel::asset

        begin
            push.{asset} exec.asset::validate_non_fungible_asset
        end
        ",
        asset = prepare_word(&encoded)
    );

    let process = tx_context.execute_code(&code).unwrap();
    assert_eq!(process.get_stack_word(0), encoded);
}
