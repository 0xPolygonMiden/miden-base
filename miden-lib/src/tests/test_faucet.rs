use miden_objects::assets::FungibleAsset;
use mock::{
    constants::{
        non_fungible_asset, non_fungible_asset_2, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1, CONSUMED_ASSET_1_AMOUNT, FUNGIBLE_ASSET_AMOUNT,
        FUNGIBLE_FAUCET_INITIAL_BALANCE,
    },
    mock::{account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs},
    prepare_transaction,
    procedures::prepare_word,
    run_tx,
};

use super::{build_tx_inputs, ONE};
use crate::memory::FAUCET_STORAGE_DATA_SLOT;

// FUNGIBLE FAUCET MINT TESTS
// ================================================================================================

#[test]
fn test_mint_fungible_asset_succeeds() {
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::FungibleFaucet {
            acct_id: ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
    );

    let code = format!(
        "
        use.miden::sat::internal::account
        use.miden::sat::internal::asset_vault
        use.miden::sat::internal::layout
        use.miden::sat::internal::prologue
        use.miden::sat::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN}
            exec.faucet::mint

            # assert the correct asset is returned
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN}
            assert_eqw

            # assert the input vault has been updated
            exec.layout::get_input_vault_root_ptr
            push.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN}
            exec.asset_vault::get_balance
            push.{FUNGIBLE_ASSET_AMOUNT} assert_eq

            # assert the faucet storage has been updated
            push.{FAUCET_STORAGE_DATA_SLOT}
            exec.account::get_item
            push.{expected_final_storage_amount}
            assert_eq
        end
        ",
        expected_final_storage_amount = FUNGIBLE_FAUCET_INITIAL_BALANCE + FUNGIBLE_ASSET_AMOUNT
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let _process = run_tx(program, stack_inputs, advice_provider).unwrap();
}

#[test]
fn test_mint_fungible_asset_fails_not_faucet_account() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN}
            exec.faucet::mint
        end
        "
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);
    assert!(process.is_err());
}

#[test]
fn test_mint_fungible_asset_inconsistent_faucet_id() {
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::FungibleFaucet {
            acct_id: ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
    );

    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1}
            exec.faucet::mint
        end
        ",
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_err());
}

#[test]
fn test_mint_fungible_asset_fails_saturate_max_amount() {
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::FungibleFaucet {
            acct_id: ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
    );

    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{saturating_amount}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN}
            exec.faucet::mint
        end
        ",
        saturating_amount = FungibleAsset::MAX_AMOUNT - FUNGIBLE_FAUCET_INITIAL_BALANCE + 1
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_err());
}

// NON-FUNGIBLE FAUCET MINT TESTS
// ================================================================================================

// TODO: reenable once storage map support is implemented
#[ignore]
#[test]
fn test_mint_non_fungible_asset_succeeds() {
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::NonFungibleFaucet {
            acct_id: ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
    );
    let non_fungible_asset = non_fungible_asset(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);

    let code = format!(
        "
        use.std::collections::smt

        use.miden::sat::internal::account
        use.miden::sat::internal::asset_vault
        use.miden::sat::internal::layout
        use.miden::sat::internal::prologue
        use.miden::sat::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{non_fungible_asset}
            exec.faucet::mint

            # assert the correct asset is returned
            push.{non_fungible_asset}
            assert_eqw

            # assert the input vault has been updated.
            exec.layout::get_input_vault_root_ptr
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

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let _process = run_tx(program, stack_inputs, advice_provider).unwrap();
}

#[test]
fn test_mint_non_fungible_asset_fails_not_faucet_account() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let non_fungible_asset = non_fungible_asset(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);

    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{non_fungible_asset}
            exec.faucet::mint
        end
        ",
        non_fungible_asset = prepare_word(&non_fungible_asset.into())
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_err());
}

#[test]
fn test_mint_non_fungible_asset_fails_inconsistent_faucet_id() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let non_fungible_asset = non_fungible_asset(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1);

    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{non_fungible_asset}
            exec.faucet::mint
        end
        ",
        non_fungible_asset = prepare_word(&non_fungible_asset.into())
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_err());
}

#[test]
fn test_mint_non_fungible_asset_fails_asset_already_exists() {
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::NonFungibleFaucet {
            acct_id: ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
    );
    let non_fungible_asset = non_fungible_asset_2(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);

    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{non_fungible_asset}
            exec.faucet::mint
        end
        ",
        non_fungible_asset = prepare_word(&non_fungible_asset.into())
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_err());
}

// FUNGIBLE FAUCET BURN TESTS
// ================================================================================================

#[test]
fn test_burn_fungible_asset_succeeds() {
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::FungibleFaucet {
            acct_id: ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
    );

    let code = format!(
        "
        use.miden::sat::internal::account
        use.miden::sat::internal::asset_vault
        use.miden::sat::internal::layout
        use.miden::sat::internal::prologue
        use.miden::sat::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1}
            exec.faucet::burn

            # assert the correct asset is returned
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1}
            assert_eqw

            # assert the input vault has been updated
            exec.layout::get_input_vault_root_ptr
            push.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1}
            exec.asset_vault::get_balance
            push.{final_input_vault_asset_amount} assert_eq

            # assert the faucet storage has been updated
            push.{FAUCET_STORAGE_DATA_SLOT}
            exec.account::get_item
            push.{expected_final_storage_amount}
            assert_eq
        end
        ",
        final_input_vault_asset_amount = CONSUMED_ASSET_1_AMOUNT - FUNGIBLE_ASSET_AMOUNT,
        expected_final_storage_amount = FUNGIBLE_FAUCET_INITIAL_BALANCE - FUNGIBLE_ASSET_AMOUNT
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let _process = run_tx(program, stack_inputs, advice_provider).unwrap();
}

#[test]
fn test_burn_fungible_asset_fails_not_faucet_account() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1}
            exec.faucet::burn
        end
        "
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_err());
}

#[test]
fn test_burn_fungible_asset_inconsistent_faucet_id() {
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::FungibleFaucet {
            acct_id: ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
    );

    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1}
            exec.faucet::burn
        end
        ",
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);
    assert!(process.is_err());
}

#[test]
fn test_burn_fungible_asset_insufficient_input_amount() {
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::FungibleFaucet {
            acct_id: ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
    );

    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{saturating_amount}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1}
            exec.faucet::burn
        end
        ",
        saturating_amount = CONSUMED_ASSET_1_AMOUNT + 1
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_err());
}

// NON-FUNGIBLE FAUCET BURN TESTS
// ================================================================================================

// TODO: reenable once storage map support is implemented
#[ignore]
#[test]
fn test_burn_non_fungible_asset_succeeds() {
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::NonFungibleFaucet {
            acct_id: ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::TooManyNonFungibleInput,
    );
    let non_fungible_asset_burnt = non_fungible_asset_2(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);

    let code = format!(
        "
        use.std::collections::smt

        use.miden::sat::internal::account
        use.miden::sat::internal::asset_vault
        use.miden::sat::internal::layout
        use.miden::sat::internal::prologue
        use.miden::sat::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{non_fungible_asset}
            exec.faucet::burn

            # assert the correct asset is returned
            push.{non_fungible_asset}
            assert_eqw

            # assert the input vault has been updated.
            exec.layout::get_input_vault_root_ptr
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

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let _process = run_tx(program, stack_inputs, advice_provider).unwrap();
}

#[test]
fn test_burn_non_fungible_asset_fails_does_not_exist() {
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::NonFungibleFaucet {
            acct_id: ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::TooManyNonFungibleInput,
    );
    let non_fungible_asset_burnt = non_fungible_asset(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);

    let code = format!(
        "
        use.std::collections::smt

        use.miden::sat::internal::account
        use.miden::sat::internal::asset_vault
        use.miden::sat::internal::layout
        use.miden::sat::internal::prologue
        use.miden::sat::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{non_fungible_asset}
            exec.faucet::burn
        end
        ",
        non_fungible_asset = prepare_word(&non_fungible_asset_burnt.into())
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_err());
}

#[test]
fn test_burn_non_fungible_asset_fails_not_faucet_account() {
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::TooManyNonFungibleInput,
    );
    let non_fungible_asset_burnt = non_fungible_asset_2(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);

    let code = format!(
        "
        use.std::collections::smt

        use.miden::sat::internal::account
        use.miden::sat::internal::asset_vault
        use.miden::sat::internal::layout
        use.miden::sat::internal::prologue
        use.miden::sat::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{non_fungible_asset}
            exec.faucet::burn
        end
        ",
        non_fungible_asset = prepare_word(&non_fungible_asset_burnt.into())
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_err());
}

#[test]
fn test_burn_non_fungible_asset_fails_inconsistent_faucet_id() {
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::NonFungibleFaucet {
            acct_id: ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::TooManyNonFungibleInput,
    );
    let non_fungible_asset_burnt = non_fungible_asset_2(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1);

    let code = format!(
        "
        use.std::collections::smt

        use.miden::sat::internal::account
        use.miden::sat::internal::asset_vault
        use.miden::sat::internal::layout
        use.miden::sat::internal::prologue
        use.miden::sat::faucet

        begin
            # mint asset
            exec.prologue::prepare_transaction
            push.{non_fungible_asset}
            exec.faucet::burn
        end
        ",
        non_fungible_asset = prepare_word(&non_fungible_asset_burnt.into())
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_err());
}

// GET TOTAL ISSUANCE TESTS
// ================================================================================================

#[test]
fn test_get_total_issuance_succeeds() {
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::FungibleFaucet {
            acct_id: ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::TooManyNonFungibleInput,
    );

    let code = format!(
        "\
    use.miden::sat::internal::prologue
    use.miden::sat::faucet

    begin
        # prepare the transaction
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

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let _process = run_tx(program, stack_inputs, advice_provider).unwrap();
}
