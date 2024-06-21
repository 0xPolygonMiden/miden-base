use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
};

use miden_objects::{
    accounts::{
        Account, AccountCode, AccountId, AccountStorage, AccountStorageType, AccountType, SlotItem,
    },
    assembly::ModuleAst,
    AccountError, Word,
};

use super::{AuthScheme, TransactionKernel};

// BASIC WALLET
// ================================================================================================

/// Creates a new account with basic wallet interface, the specified authentication scheme and the account storage type.
/// Basic wallets can be specified to have either mutable or immutable code.
///
/// The basic wallet interface exposes two procedures:
/// - `receive_asset`, which can be used to add an asset to the account.
/// - `send_asset`, which can be used to remove an asset from the account and put into a note
///    addressed to the specified recipient.
///
/// Both methods require authentication. The authentication procedure is defined by the specified
/// authentication scheme. Public key information for the scheme is stored in the account storage
/// at slot 0.
pub fn create_basic_wallet(
    init_seed: [u8; 32],
    auth_scheme: AuthScheme,
    account_type: AccountType,
    account_storage_type: AccountStorageType,
) -> Result<(Account, Word), AccountError> {
    if matches!(account_type, AccountType::FungibleFaucet | AccountType::NonFungibleFaucet) {
        return Err(AccountError::AccountIdInvalidFieldElement(
            "Basic wallet accounts cannot have a faucet account type".to_string(),
        ));
    }

    let (auth_scheme_procedure, storage_slot_0_data): (&str, Word) = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => ("basic::auth_tx_rpo_falcon512", pub_key.into()),
    };

    let account_code_string: String = format!(
        "
        use.miden::contracts::wallets::basic->basic_wallet
        use.miden::contracts::auth::basic

        export.basic_wallet::receive_asset
        export.basic_wallet::send_asset
        export.{auth_scheme_procedure}
    "
    );
    let account_code_src: &str = &account_code_string;

    let account_code_ast = ModuleAst::parse(account_code_src)
        .map_err(|e| AccountError::AccountCodeAssemblerError(e.into()))?;
    let account_assembler = TransactionKernel::assembler();
    let account_code = AccountCode::new(account_code_ast.clone(), &account_assembler)?;

    let account_storage =
        AccountStorage::new(vec![SlotItem::new_value(0, 0, storage_slot_0_data)], BTreeMap::new())?;

    let account_seed = AccountId::get_account_seed(
        init_seed,
        account_type,
        account_storage_type,
        account_code.root(),
        account_storage.root(),
    )?;

    Ok((Account::new(account_seed, account_code, account_storage)?, account_seed))
}

// CUSTOM WALLET
// ================================================================================================

/// Creates a new account with a custom wallet interface, the specified authentication scheme, and the account storage type.
///
pub fn create_wallet_with_custom_code(
    init_seed: [u8; 32],
    auth_scheme: AuthScheme,
    account_type: AccountType,
    account_storage_type: AccountStorageType,
    custom_code: String,
) -> Result<(Account, Word), AccountError> {
    if matches!(account_type, AccountType::FungibleFaucet | AccountType::NonFungibleFaucet) {
        return Err(AccountError::AccountIdInvalidFieldElement(
            "Basic wallet accounts cannot have a faucet account type".to_string(),
        ));
    }

    let (auth_scheme_procedure, storage_slot_0_data): (&str, Word) = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => ("basic::auth_tx_rpo_falcon512", pub_key.into()),
    };

    let account_code_string: String = format!(
        "
        {custom_code}

        export.{auth_scheme_procedure}
    "
    );
    let account_code_src: &str = &account_code_string;

    let account_code_ast = ModuleAst::parse(account_code_src)
        .map_err(|e| AccountError::AccountCodeAssemblerError(e.into()))?;
    let account_assembler = TransactionKernel::assembler();
    let account_code = AccountCode::new(account_code_ast.clone(), &account_assembler)?;

    let account_storage =
        AccountStorage::new(vec![SlotItem::new_value(0, 0, storage_slot_0_data)], BTreeMap::new())?;

    let account_seed = AccountId::get_account_seed(
        init_seed,
        account_type,
        account_storage_type,
        account_code.root(),
        account_storage.root(),
    )?;

    Ok((Account::new(account_seed, account_code, account_storage)?, account_seed))
}
