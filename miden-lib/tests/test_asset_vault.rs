pub mod common;
use common::{
    data::{
        mock_inputs, AccountStatus, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
    },
    prepare_transaction, run_tx, AccountId, MemAdviceProvider, ONE,
};
use crypto::StarkField;

use crate::common::procedures::prepare_word;

#[test]
fn test_get_balance() {
    let (account, block_header, chain, notes) = mock_inputs(AccountStatus::Existing, None, None);

    let faucet_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::account

        begin
            exec.prologue::prepare_transaction
            push.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN}
            exec.account::get_balance
        end
    "
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, &code, "", None, None);

    let process = run_tx(
        transaction.tx_program().clone(),
        transaction.stack_inputs(),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    )
    .unwrap();

    assert_eq!(
        process.stack.get(0).as_int(),
        transaction.account().vault().get_balance(faucet_id).unwrap()
    );
}

#[test]
fn test_get_balance_non_fungible_fails() {
    let (account, block_header, chain, notes) = mock_inputs(AccountStatus::Existing, None, None);

    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::account

        begin
            exec.prologue::prepare_transaction
            push.{ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN}
            exec.account::get_balance
        end
    "
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, &code, "", None, None);

    let process = run_tx(
        transaction.tx_program().clone(),
        transaction.stack_inputs(),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    );

    assert!(process.is_err());
}

#[test]
fn test_has_non_fungible_asset() {
    let (account, block_header, chain, notes) = mock_inputs(AccountStatus::Existing, None, None);

    let non_fungible_asset = account.vault().assets().next().unwrap();

    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::account

        begin
            exec.prologue::prepare_transaction
            push.{non_fungible_asset_key}
            exec.account::has_non_fungible_asset
        end
    ",
        non_fungible_asset_key = prepare_word(&non_fungible_asset.vault_key())
    );

    let inputs =
        prepare_transaction(account, None, block_header, chain, notes, &code, "", None, None);

    let process = run_tx(
        inputs.tx_program().clone(),
        inputs.stack_inputs(),
        MemAdviceProvider::from(inputs.advice_provider_inputs()),
    )
    .unwrap();

    assert_eq!(process.stack.get(0), ONE);
}
