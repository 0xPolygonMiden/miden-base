pub mod common;
use common::{
    data::{mock_inputs, AccountStatus},
    prepare_transaction, run_tx, MemAdviceProvider,
};
use vm_core::StackInputs;

#[test]
// Doesn't work yet. WIP. It compiles though.
fn test_add_asset_via_wallet() {
    let (account, block_header, chain, notes) = mock_inputs(AccountStatus::Existing);

    let code = format!(
        "
    use.miden::sat::miden_wallet

    # add the asset
    begin
        exec.miden_wallet::receive_asset
    end
    "
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, &code, "", None, None);

    let _process = run_tx(
        transaction.tx_program().clone(),
        StackInputs::from(transaction.stack_inputs()),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    )
    .unwrap();
}
