use miden_objects::{
    accounts::account_id::testing::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1,
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1,
    },
    assets::{FungibleAsset, NonFungibleAsset},
    testing::{
        constants::{
            CONSUMED_ASSET_1_AMOUNT, FUNGIBLE_ASSET_AMOUNT, FUNGIBLE_FAUCET_INITIAL_BALANCE,
            NON_FUNGIBLE_ASSET_DATA, NON_FUNGIBLE_ASSET_DATA_2,
        },
        prepare_word,
        storage::FAUCET_STORAGE_DATA_SLOT,
    },
    FieldElement,
};
use vm_processor::Felt;

use super::ONE;
use crate::{
    assert_execution_error,
    errors::tx_kernel_errors::{
        ERR_FAUCET_ISSUANCE_OVERFLOW, ERR_FAUCET_NONEXISTING_TOKEN,
        ERR_FAUCET_NON_FUNGIBLE_ALREADY_EXISTS, ERR_FAUCET_NON_FUNGIBLE_BURN_WRONG_TYPE,
        ERR_FUNGIBLE_ASSET_MISMATCH, ERR_NON_FUNGIBLE_ASSET_MISMATCH,
        ERR_VAULT_FUNGIBLE_AMOUNT_UNDERFLOW,
    },
    testing::TransactionContextBuilder,
};

// FUNGIBLE FAUCET MINT TESTS
// ================================================================================================

#[test]
fn test_mint_fungible_asset_succeeds() {
    let tx_context = TransactionContextBuilder::with_fungible_faucet(
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ONE,
        Felt::new(FUNGIBLE_FAUCET_INITIAL_BALANCE),
    )
    .build();

    let code = format!(
        "
        use.test::account
        use.kernel::asset_vault
        use.kernel::memory
        use.kernel::prologue

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN}
            call.account::mint

            # assert the correct asset is returned
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN}
            assert_eqw

            # assert the input vault has been updated
            exec.memory::get_input_vault_root_ptr
            push.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN}
            exec.asset_vault::get_balance
            push.{FUNGIBLE_ASSET_AMOUNT} assert_eq

            # assert the faucet storage has been updated
            push.{FAUCET_STORAGE_DATA_SLOT}
            call.account::get_item
            push.{expected_final_storage_amount}
            assert_eq
        end
        ",
        expected_final_storage_amount = FUNGIBLE_FAUCET_INITIAL_BALANCE + FUNGIBLE_ASSET_AMOUNT
    );

    tx_context.execute_code(&code).unwrap();
}

#[test]
fn test_mint_fungible_asset_fails_not_faucet_account() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let code = format!(
        "
        use.kernel::prologue
        use.test::account

        begin
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN}
            call.account::mint
        end
        "
    );

    let process = tx_context.execute_code(&code);

    assert_execution_error!(process, ERR_FUNGIBLE_ASSET_MISMATCH);
}

#[test]
fn test_mint_fungible_asset_inconsistent_faucet_id() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let code = format!(
        "
        use.kernel::prologue
        use.test::account

        begin
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1}
            call.account::mint
        end
        ",
    );

    let process = tx_context.execute_code(&code);

    assert_execution_error!(process, ERR_FUNGIBLE_ASSET_MISMATCH);
}

#[test]
fn test_mint_fungible_asset_fails_saturate_max_amount() {
    let tx_context = TransactionContextBuilder::with_fungible_faucet(
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        Felt::ONE,
        Felt::new(FUNGIBLE_FAUCET_INITIAL_BALANCE),
    )
    .build();

    let code = format!(
        "
        use.kernel::prologue
        use.test::account

        begin
            exec.prologue::prepare_transaction
            push.{saturating_amount}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN}
            call.account::mint
        end
        ",
        saturating_amount = FungibleAsset::MAX_AMOUNT - FUNGIBLE_FAUCET_INITIAL_BALANCE + 1
    );

    let process = tx_context.execute_code(&code);

    assert_execution_error!(process, ERR_FAUCET_ISSUANCE_OVERFLOW);
}

// NON-FUNGIBLE FAUCET MINT TESTS
// ================================================================================================

// TODO: reenable once storage map support is implemented
#[ignore]
#[test]
fn test_mint_non_fungible_asset_succeeds() {
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
        use.std::collections::smt

        use.kernel::account
        use.kernel::asset_vault
        use.kernel::memory
        use.kernel::prologue
        use.miden::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{non_fungible_asset}
            exec.faucet::mint

            # assert the correct asset is returned
            push.{non_fungible_asset}
            assert_eqw

            # assert the input vault has been updated.
            exec.memory::get_input_vault_root_ptr
            push.{non_fungible_asset}
            exec.asset_vault::has_non_fungible_asset
            assert

            # assert the non-fungible asset has been added to the faucet smt
            push.{FAUCET_STORAGE_DATA_SLOT}
            exec.account::get_item
            push.{non_fungible_asset}
            exec.smt::get
            push.{non_fungible_asset}
            assert_eqw
        end
        ",
        non_fungible_asset = prepare_word(&non_fungible_asset.into())
    );

    tx_context.execute_code(&code).unwrap();
}

#[test]
fn test_mint_non_fungible_asset_fails_not_faucet_account() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let non_fungible_asset =
        NonFungibleAsset::mock(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &[1, 2, 3, 4]);

    let code = format!(
        "
        use.kernel::prologue
        use.test::account

        begin
            exec.prologue::prepare_transaction
            push.{non_fungible_asset}
            call.account::mint
        end
        ",
        non_fungible_asset = prepare_word(&non_fungible_asset.into())
    );

    let process = tx_context.execute_code(&code);

    assert_execution_error!(process, ERR_NON_FUNGIBLE_ASSET_MISMATCH);
}

#[test]
fn test_mint_non_fungible_asset_fails_inconsistent_faucet_id() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let non_fungible_asset =
        NonFungibleAsset::mock(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1, &[1, 2, 3, 4]);

    let code = format!(
        "
        use.kernel::prologue
        use.test::account

        begin
            exec.prologue::prepare_transaction
            push.{non_fungible_asset}
            call.account::mint
        end
        ",
        non_fungible_asset = prepare_word(&non_fungible_asset.into())
    );

    let process = tx_context.execute_code(&code);

    assert_execution_error!(process, ERR_NON_FUNGIBLE_ASSET_MISMATCH);
}

#[test]
fn test_mint_non_fungible_asset_fails_asset_already_exists() {
    let tx_context = TransactionContextBuilder::with_non_fungible_faucet(
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        ONE,
        false,
    )
    .build();

    let non_fungible_asset =
        NonFungibleAsset::mock(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &NON_FUNGIBLE_ASSET_DATA_2);

    let code = format!(
        "
        use.kernel::prologue
        use.test::account

        begin
            exec.prologue::prepare_transaction
            push.{non_fungible_asset}
            call.account::mint
        end
        ",
        non_fungible_asset = prepare_word(&non_fungible_asset.into())
    );

    let process = tx_context.execute_code(&code);

    assert_execution_error!(process, ERR_FAUCET_NON_FUNGIBLE_ALREADY_EXISTS);
}

// FUNGIBLE FAUCET BURN TESTS
// ================================================================================================

#[test]
fn test_burn_fungible_asset_succeeds() {
    let tx_context = TransactionContextBuilder::with_fungible_faucet(
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1,
        ONE,
        Felt::new(FUNGIBLE_FAUCET_INITIAL_BALANCE),
    )
    .with_mock_notes_preserved()
    .build();

    let code = format!(
        "
        use.test::account
        use.kernel::asset_vault
        use.kernel::memory
        use.kernel::prologue

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1}
            call.account::burn
            # assert the correct asset is returned
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1}
            assert_eqw

            # assert the input vault has been updated
            exec.memory::get_input_vault_root_ptr

            push.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1}
            exec.asset_vault::get_balance
            
            push.{final_input_vault_asset_amount} assert_eq
            
            # assert the faucet storage has been updated
            push.{FAUCET_STORAGE_DATA_SLOT}
            call.account::get_item
            push.{expected_final_storage_amount}
            assert_eq
        end
        ",
        final_input_vault_asset_amount = CONSUMED_ASSET_1_AMOUNT - FUNGIBLE_ASSET_AMOUNT,
        expected_final_storage_amount = FUNGIBLE_FAUCET_INITIAL_BALANCE - FUNGIBLE_ASSET_AMOUNT
    );

    tx_context.execute_code(&code).unwrap();
}

#[test]
fn test_burn_fungible_asset_fails_not_faucet_account() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let code = format!(
        "
        use.kernel::prologue
        use.test::account

        begin
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1}
            call.account::burn
        end
        "
    );

    let process = tx_context.execute_code(&code);

    assert_execution_error!(process, ERR_FUNGIBLE_ASSET_MISMATCH);
}

#[test]
fn test_burn_fungible_asset_inconsistent_faucet_id() {
    let tx_context = TransactionContextBuilder::with_non_fungible_faucet(
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ONE,
        false,
    )
    .build();

    let code = format!(
        "
        use.kernel::prologue
        use.test::account

        begin
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1}
            call.account::burn
        end
        ",
    );

    let process = tx_context.execute_code(&code);

    assert_execution_error!(process, ERR_FUNGIBLE_ASSET_MISMATCH);
}

#[test]
fn test_burn_fungible_asset_insufficient_input_amount() {
    let tx_context = TransactionContextBuilder::with_fungible_faucet(
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1,
        ONE,
        Felt::new(FUNGIBLE_FAUCET_INITIAL_BALANCE),
    )
    .build();

    let code = format!(
        "
        use.kernel::prologue
        use.test::account

        begin
            exec.prologue::prepare_transaction
            push.{saturating_amount}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1}
            call.account::burn
        end
        ",
        saturating_amount = CONSUMED_ASSET_1_AMOUNT + 1
    );

    let process = tx_context.execute_code(&code);

    assert_execution_error!(process, ERR_VAULT_FUNGIBLE_AMOUNT_UNDERFLOW);
}

// NON-FUNGIBLE FAUCET BURN TESTS
// ================================================================================================

// TODO: reenable once storage map support is implemented
#[ignore]
#[test]
fn test_burn_non_fungible_asset_succeeds() {
    let tx_context = TransactionContextBuilder::with_non_fungible_faucet(
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        ONE,
        false,
    )
    .build();

    let non_fungible_asset_burnt =
        NonFungibleAsset::mock(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &[1, 2, 3]);

    let code = format!(
        "
        use.std::collections::smt

        use.kernel::account
        use.kernel::asset_vault
        use.kernel::memory
        use.kernel::prologue
        use.miden::faucet

        begin
            # burn asset
            exec.prologue::prepare_transaction
            push.{non_fungible_asset}
            exec.faucet::burn

            # assert the correct asset is returned
            push.{non_fungible_asset}
            assert_eqw

            # assert the input vault has been updated.
            exec.memory::get_input_vault_root_ptr
            push.{non_fungible_asset}
            exec.asset_vault::has_non_fungible_asset
            not assert

            # assert the non-fungible asset has been removed from the faucet smt
            push.{FAUCET_STORAGE_DATA_SLOT}
            exec.account::get_item
            push.{non_fungible_asset}
            exec.smt::get
            padw
            assert_eqw
        end
        ",
        non_fungible_asset = prepare_word(&non_fungible_asset_burnt.into())
    );

    tx_context.execute_code(&code).unwrap();
}

#[test]
fn test_burn_non_fungible_asset_fails_does_not_exist() {
    let tx_context = TransactionContextBuilder::with_non_fungible_faucet(
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        ONE,
        false,
    )
    .build();

    let non_fungible_asset_burnt =
        NonFungibleAsset::mock(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &[1, 2, 3]);

    let code = format!(
        "
        use.std::collections::smt

        #use.kernel::account
        use.kernel::asset_vault
        use.kernel::memory
        use.kernel::prologue
        use.test::account

        begin
            # burn asset
            exec.prologue::prepare_transaction
            push.{non_fungible_asset}
            call.account::burn
        end
        ",
        non_fungible_asset = prepare_word(&non_fungible_asset_burnt.into())
    );

    let process = tx_context.execute_code(&code);

    assert_execution_error!(process, ERR_FAUCET_NONEXISTING_TOKEN);
}

#[test]
fn test_burn_non_fungible_asset_fails_not_faucet_account() {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE).build();

    let non_fungible_asset_burnt =
        NonFungibleAsset::mock(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, &[1, 2, 3]);

    let code = format!(
        "
        use.std::collections::smt

        use.kernel::asset_vault
        use.kernel::memory
        use.kernel::prologue
        use.test::account

        begin
            # burn asset
            exec.prologue::prepare_transaction
            push.{non_fungible_asset}
            call.account::burn
        end
        ",
        non_fungible_asset = prepare_word(&non_fungible_asset_burnt.into())
    );

    let process = tx_context.execute_code(&code);

    assert_execution_error!(process, ERR_FAUCET_NON_FUNGIBLE_BURN_WRONG_TYPE);
}

#[test]
fn test_burn_non_fungible_asset_fails_inconsistent_faucet_id() {
    let non_fungible_asset_burnt =
        NonFungibleAsset::mock(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1, &[1, 2, 3]);

    let tx_context = TransactionContextBuilder::with_non_fungible_faucet(
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        ONE,
        false,
    )
    .build();

    let code = format!(
        "
        use.std::collections::smt

        use.kernel::asset_vault
        use.kernel::memory
        use.kernel::prologue
        use.test::account

        begin
            # burn asset
            exec.prologue::prepare_transaction
            push.{non_fungible_asset}
            call.account::burn
        end
        ",
        non_fungible_asset = prepare_word(&non_fungible_asset_burnt.into())
    );

    let process = tx_context.execute_code(&code);

    assert_execution_error!(process, ERR_FAUCET_NONEXISTING_TOKEN);
}

// GET TOTAL ISSUANCE TESTS
// ================================================================================================

#[test]
fn test_get_total_issuance_succeeds() {
    let tx_context = TransactionContextBuilder::with_fungible_faucet(
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ONE,
        Felt::new(FUNGIBLE_FAUCET_INITIAL_BALANCE),
    )
    .build();

    let code = format!(
        "
        use.kernel::prologue
        use.miden::faucet

        begin
            exec.prologue::prepare_transaction

            # get the fungible faucet balance
            exec.faucet::get_total_issuance
            # => [total_issuance]

            # assert the correct balance is returned
            push.{FUNGIBLE_FAUCET_INITIAL_BALANCE} assert_eq
            # => []
        end
        ",
    );

    tx_context.execute_code(&code).unwrap();
}
