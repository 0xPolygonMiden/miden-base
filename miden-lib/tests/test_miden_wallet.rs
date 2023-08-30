pub mod common;
use assembly::ast::{ModuleAst, ProgramAst};
use assembly::ast::{ModuleAst, ProgramAst};
use common::{
    data::{mock_inputs, AccountStatus},
    prepare_transaction, run_tx, MemAdviceProvider,
};
use crypto::{Felt, Word, ONE};
use vm_core::StackInputs;

use miden_objects::{
    assets::{Asset, FungibleAsset},
    mock::{
        assembler, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN, ACCOUNT_ID_SENDER,
    },
    notes::{Note, NoteScript},
    Account, AccountCode, AccountId, AccountStorage, AccountVault,
};

#[test]
// Testing the basic Miden wallet - receiving an asset
fn test_add_asset_via_wallet() {
    
    // Mock data
    // We need an account and a note carrying an asset. 
    
    let tx_script_code = format!(
        "
    use.miden::wallets::basic->wallet
    use.miden::eoa::basic->authentication
    use.miden::wallets::basic->wallet
    use.miden::eoa::basic->authentication

    # add the asset
    begin
        exec.wallet::receive_asset
        exec.authentication::auth_tx
        exec.wallet::receive_asset
        exec.authentication::auth_tx
    end
    "
    );

    let transaction = prepare_transaction(
        account,
        None,
        block_header,
        chain,
        notes,
        &tx_script_code,
        "",
        None,
        None,
    );
    let transaction = prepare_transaction(
        account,
        None,
        block_header,
        chain,
        notes,
        &tx_script_code,
        "",
        None,
        None,
    );

    let _process = run_tx(
        transaction.tx_program().clone(),
        StackInputs::from(transaction.stack_inputs()),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    )
    .unwrap();
}
