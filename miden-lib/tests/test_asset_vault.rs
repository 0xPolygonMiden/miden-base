pub mod common;
use common::{
    data::{
        mock_inputs, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
    },
    run_within_tx_kernel, AccountId, MemAdviceProvider, ONE,
};
use crypto::StarkField;

use crate::common::procedures::prepare_word;

#[test]
fn test_get_balance() {
    let faucet_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
    let inputs = mock_inputs();

    let code = format!(
        "
        use.miden::sat::prologue
        use.miden::sat::account_vault

        begin
            exec.prologue::prepare_transaction
            push.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN}
            exec.account_vault::get_balance
        end
    "
    );

    let process = run_within_tx_kernel(
        "",
        &code,
        inputs.stack_inputs(),
        MemAdviceProvider::from(inputs.advice_provider_inputs()),
        None,
        None,
    )
    .unwrap();

    assert_eq!(
        process.stack.get(0).as_int(),
        inputs.account().vault().get_balance(faucet_id).unwrap()
    );
}

#[test]
fn test_get_balance_non_fungible_fails() {
    let inputs = mock_inputs();

    let code = format!(
        "
        use.miden::sat::prologue
        use.miden::sat::account_vault

        begin
            exec.prologue::prepare_transaction
            push.{ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN}
            exec.account_vault::get_balance
        end
    "
    );

    let process = run_within_tx_kernel(
        "",
        &code,
        inputs.stack_inputs(),
        MemAdviceProvider::from(inputs.advice_provider_inputs()),
        None,
        None,
    );

    assert!(process.is_err());
}

#[test]
fn test_has_non_fungible_asset() {
    let inputs = mock_inputs();
    let non_fungible_asset = inputs.account().vault().assets().next().unwrap();

    let code = format!(
        "
        use.miden::sat::prologue
        use.miden::sat::account_vault

        begin
            exec.prologue::prepare_transaction
            push.{non_fungible_asset_key}
            exec.account_vault::has_non_fungible_asset
        end
    ",
        non_fungible_asset_key = prepare_word(&non_fungible_asset.vault_key())
    );

    let process = run_within_tx_kernel(
        "",
        &code,
        inputs.stack_inputs(),
        MemAdviceProvider::from(inputs.advice_provider_inputs()),
        None,
        None,
    )
    .unwrap();

    assert_eq!(process.stack.get(0), ONE);
}
