use miden_objects::{
    accounts::AccountId,
    assets::{Asset, FungibleAsset, NonFungibleAsset, NonFungibleAssetDetails},
    StarkField,
};
use mock::{
    constants::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        FUNGIBLE_ASSET_AMOUNT, NON_FUNGIBLE_ASSET_DATA,
    },
    mock::{account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs},
    prepare_transaction,
    procedures::prepare_word,
    run_tx,
};

use super::{build_tx_inputs, ContextId, Felt, ProcessState, Word, ONE, ZERO};
use crate::transaction::memory;

#[test]
fn test_get_balance() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let faucet_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::account

        begin
            exec.prologue::prepare_transaction
            push.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN}
            exec.account::get_balance
        end
    "
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider).unwrap();

    assert_eq!(
        process.stack.get(0).as_int(),
        transaction.account().vault().get_balance(faucet_id).unwrap()
    );
}

#[test]
fn test_get_balance_non_fungible_fails() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::account

        begin
            exec.prologue::prepare_transaction
            push.{ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN}
            exec.account::get_balance
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
fn test_has_non_fungible_asset() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let non_fungible_asset = account.vault().assets().next().unwrap();

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::account

        begin
            exec.prologue::prepare_transaction
            push.{non_fungible_asset_key}
            exec.account::has_non_fungible_asset
        end
    ",
        non_fungible_asset_key = prepare_word(&non_fungible_asset.vault_key())
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider).unwrap();

    assert_eq!(process.stack.get(0), ONE);
}

#[test]
fn test_add_fungible_asset_success() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let mut account_vault = account.vault().clone();

    let faucet_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
    let amount = FungibleAsset::MAX_AMOUNT - FUNGIBLE_ASSET_AMOUNT;
    let add_fungible_asset =
        Asset::try_from([Felt::new(amount), ZERO, ZERO, faucet_id.into()]).unwrap();

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::account

        begin
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET}
            exec.account::add_asset
        end
    ",
        FUNGIBLE_ASSET = prepare_word(&add_fungible_asset.into())
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider).unwrap();

    assert_eq!(
        process.stack.get_word(0),
        Into::<Word>::into(account_vault.add_asset(add_fungible_asset).unwrap())
    );

    assert_eq!(
        process.get_mem_value(ContextId::root(), memory::ACCT_VAULT_ROOT_PTR).unwrap(),
        *account_vault.commitment()
    );
}

#[test]
fn test_add_non_fungible_asset_fail_overflow() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let mut account_vault = account.vault().clone();

    let faucet_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
    let amount = FungibleAsset::MAX_AMOUNT - FUNGIBLE_ASSET_AMOUNT + 1;
    let add_fungible_asset =
        Asset::try_from([Felt::new(amount), ZERO, ZERO, faucet_id.into()]).unwrap();

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::account

        begin
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET}
            exec.account::add_asset
        end
    ",
        FUNGIBLE_ASSET = prepare_word(&add_fungible_asset.into())
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_err());
    assert!(account_vault.add_asset(add_fungible_asset).is_err());
}

#[test]
fn test_add_non_fungible_asset_success() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let faucet_id: AccountId = ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
    let mut account_vault = account.vault().clone();
    let add_non_fungible_asset = Asset::NonFungible(
        NonFungibleAsset::new(
            &NonFungibleAssetDetails::new(faucet_id, vec![1, 2, 3, 4, 5, 6, 7, 8]).unwrap(),
        )
        .unwrap(),
    );

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::account

        begin
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET}
            exec.account::add_asset
        end
    ",
        FUNGIBLE_ASSET = prepare_word(&add_non_fungible_asset.into())
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider).unwrap();

    assert_eq!(
        process.stack.get_word(0),
        Into::<Word>::into(account_vault.add_asset(add_non_fungible_asset).unwrap())
    );

    assert_eq!(
        process.get_mem_value(ContextId::root(), memory::ACCT_VAULT_ROOT_PTR).unwrap(),
        *account_vault.commitment()
    );
}

#[test]
fn test_add_non_fungible_asset_fail_duplicate() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let faucet_id: AccountId = ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
    let mut account_vault = account.vault().clone();
    let non_fungible_asset_details =
        NonFungibleAssetDetails::new(faucet_id, NON_FUNGIBLE_ASSET_DATA.to_vec()).unwrap();
    let non_fungible_asset =
        Asset::NonFungible(NonFungibleAsset::new(&non_fungible_asset_details).unwrap());

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::account

        begin
            exec.prologue::prepare_transaction
            push.{NON_FUNGIBLE_ASSET}
            exec.account::add_asset
        end
    ",
        NON_FUNGIBLE_ASSET = prepare_word(&non_fungible_asset.into())
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_err());
    assert!(account_vault.add_asset(non_fungible_asset).is_err());
}

#[test]
fn test_remove_fungible_asset_success_no_balance_remaining() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let mut account_vault = account.vault().clone();

    let faucet_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
    let amount = FUNGIBLE_ASSET_AMOUNT;
    let remove_fungible_asset =
        Asset::try_from([Felt::new(amount), ZERO, ZERO, faucet_id.into()]).unwrap();

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::account

        begin
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET}
            exec.account::remove_asset
        end
    ",
        FUNGIBLE_ASSET = prepare_word(&remove_fungible_asset.into())
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider).unwrap();

    assert_eq!(
        process.stack.get_word(0),
        Into::<Word>::into(account_vault.remove_asset(remove_fungible_asset).unwrap())
    );

    assert_eq!(
        process.get_mem_value(ContextId::root(), memory::ACCT_VAULT_ROOT_PTR).unwrap(),
        *account_vault.commitment()
    );
}

#[test]
fn test_remove_fungible_asset_fail_remove_too_much() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let faucet_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
    let amount = FUNGIBLE_ASSET_AMOUNT + 1;
    let remove_fungible_asset =
        Asset::try_from([Felt::new(amount), ZERO, ZERO, faucet_id.into()]).unwrap();

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::account

        begin
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET}
            exec.account::remove_asset
        end
    ",
        FUNGIBLE_ASSET = prepare_word(&remove_fungible_asset.into())
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_err());
}

#[test]
fn test_remove_fungible_asset_success_balance_remaining() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
    let mut account_vault = account.vault().clone();

    let faucet_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
    let amount = FUNGIBLE_ASSET_AMOUNT - 1;
    let remove_fungible_asset =
        Asset::try_from([Felt::new(amount), ZERO, ZERO, faucet_id.into()]).unwrap();

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::account

        begin
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET}
            exec.account::remove_asset
        end
    ",
        FUNGIBLE_ASSET = prepare_word(&remove_fungible_asset.into())
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider).unwrap();

    assert_eq!(
        process.stack.get_word(0),
        Into::<Word>::into(account_vault.remove_asset(remove_fungible_asset).unwrap())
    );

    assert_eq!(
        process.get_mem_value(ContextId::root(), memory::ACCT_VAULT_ROOT_PTR).unwrap(),
        *account_vault.commitment()
    );
}

#[test]
fn test_remove_non_fungible_asset_fail_doesnt_exist() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let faucet_id: AccountId = (ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN + 1).try_into().unwrap();
    let mut account_vault = account.vault().clone();
    let non_fungible_asset_details =
        NonFungibleAssetDetails::new(faucet_id, NON_FUNGIBLE_ASSET_DATA.to_vec()).unwrap();
    let non_existent_non_fungible_asset =
        Asset::NonFungible(NonFungibleAsset::new(&non_fungible_asset_details).unwrap());

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::account

        begin
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET}
            exec.account::remove_asset
        end
    ",
        FUNGIBLE_ASSET = prepare_word(&non_existent_non_fungible_asset.into())
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider);

    assert!(process.is_err());
    assert!(account_vault.remove_asset(non_existent_non_fungible_asset).is_err());
}

#[test]
fn test_remove_non_fungible_asset_success() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let faucet_id: AccountId = ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();
    let mut account_vault = account.vault().clone();
    let non_fungible_asset_details =
        NonFungibleAssetDetails::new(faucet_id, NON_FUNGIBLE_ASSET_DATA.to_vec()).unwrap();
    let non_fungible_asset =
        Asset::NonFungible(NonFungibleAsset::new(&non_fungible_asset_details).unwrap());

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::account

        begin
            exec.prologue::prepare_transaction
            push.{FUNGIBLE_ASSET}
            exec.account::remove_asset
        end
    ",
        FUNGIBLE_ASSET = prepare_word(&non_fungible_asset.into())
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, None, &code, "", None);
    let (program, stack_inputs, advice_provider) = build_tx_inputs(&transaction);
    let process = run_tx(program, stack_inputs, advice_provider).unwrap();

    assert_eq!(
        process.stack.get_word(0),
        Into::<Word>::into(account_vault.remove_asset(non_fungible_asset).unwrap())
    );

    assert_eq!(
        process.get_mem_value(ContextId::root(), memory::ACCT_VAULT_ROOT_PTR).unwrap(),
        *account_vault.commitment()
    );
}
