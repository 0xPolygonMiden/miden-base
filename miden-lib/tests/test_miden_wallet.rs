pub mod common;
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
// Doesn't work yet. WIP. It compiles though.
fn test_add_asset_via_wallet() {
    // MOCK DATA
    // --------------------------------------------------------------------------------------------
    //let mut assembler = assembler();

    // Create assets
    //let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    //let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();

    // Create the sender, target and malicious accounts
    //let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    // let target_account_id =
    //     AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();

    // let malicious_account_id =
    //     AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN + 1).unwrap();

    // // TODO: We don't have the add_asset procedure in the assembler yet, we don't need custom code
    // const ACCOUNT_CODE_MASM: &'static str = "\
    // export.account_proc_1
    //     push.9.9.9.9
    //     dropw
    // end
    // ";
    // let account_code_ast = ModuleAst::parse(ACCOUNT_CODE_MASM).unwrap();

    // let target_account_code =
    //     AccountCode::new(target_account_id, account_code_ast.clone(), &mut assembler).unwrap();
    // let target_account = Account::new(
    //     target_account_id,
    //     AccountVault::default(),
    //     AccountStorage::default(),
    //     target_account_code.clone(),
    //     ONE,
    // );

    // let malicious_account_code =
    //     AccountCode::new(malicious_account_id, account_code_ast.clone(), &mut assembler).unwrap();
    // let malicious_account = Account::new(
    //     malicious_account_id,
    //     AccountVault::default(),
    //     AccountStorage::default(),
    //     malicious_account_code,
    //     ONE,
    // );

    // Create the note with the P2ID script w/ one asset
    // let note_program_ast =
    //     ProgramAst::parse(
    //         format!(
    //             "
    //             use.miden::wallet::basic->wallet

    //             begin                                                   # [note_inputs = target_account_id, ...]
    //                 exec.wallet::receive_asset                          # []
    //             end
    //             ",
    //         )
    //         .as_str(),
    //     )
    //     .unwrap();
    // let (note_script, _) = NoteScript::new(note_program_ast, &mut assembler).unwrap();

    // const SERIAL_NUM: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];

    // let note = Note::new(
    //     note_script.clone(),
    //     &[*target_account_id],
    //     &vec![fungible_asset_1],
    //     SERIAL_NUM,
    //     sender_account_id,
    //     ONE,
    //     None,
    // )
    // .unwrap();

    let (account, block_header, chain, notes) = mock_inputs(AccountStatus::Existing);

    let tx_script_code = format!(
        "
    use.miden::wallets::basic->wallet
    use.miden::eoa::basic->authentication

    # add the asset
    begin
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

    let _process = run_tx(
        transaction.tx_program().clone(),
        StackInputs::from(transaction.stack_inputs()),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    )
    .unwrap();
}
