use alloc::string::{String, ToString};

use miden_objects::{
    accounts::{
        Account, AccountCode, AccountId, AccountStorage, AccountStorageType, AccountType,
        StorageSlot,
    },
    AccountError, Word,
};

use super::{AuthScheme, TransactionKernel};

// BASIC WALLET
// ================================================================================================

/// Creates a new account with basic wallet interface, the specified authentication scheme and the
/// account storage type. Basic wallets can be specified to have either mutable or immutable code.
///
/// The basic wallet interface exposes two procedures:
/// - `receive_asset`, which can be used to add an asset to the account.
/// - `send_asset`, which can be used to remove an asset from the account and put into a note
///   addressed to the specified recipient.
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
        AuthScheme::RpoFalcon512 { pub_key } => ("auth_tx_rpo_falcon512", pub_key.into()),
    };

    let source_code: String = format!(
        "
        export.::miden::contracts::wallets::basic::receive_asset
        export.::miden::contracts::wallets::basic::send_asset
        export.::miden::contracts::auth::basic::{auth_scheme_procedure}
    "
    );

    let assembler = TransactionKernel::assembler();
    let account_code = AccountCode::compile(source_code, assembler)?;

    let account_storage = AccountStorage::new(vec![StorageSlot::Value(storage_slot_0_data)])?;

    let account_seed = AccountId::get_account_seed(
        init_seed,
        account_type,
        account_storage_type,
        account_code.commitment(),
        account_storage.commitment(),
    )?;

    Ok((Account::new(account_seed, account_code, account_storage)?, account_seed))
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {

    use miden_objects::{crypto::dsa::rpo_falcon512, ONE};
    use vm_processor::utils::{Deserializable, Serializable};

    use super::{create_basic_wallet, Account, AccountStorageType, AccountType, AuthScheme};

    #[test]
    fn test_create_basic_wallet() {
        let pub_key = rpo_falcon512::PublicKey::new([ONE; 4]);
        let wallet = create_basic_wallet(
            [1; 32],
            AuthScheme::RpoFalcon512 { pub_key },
            AccountType::RegularAccountImmutableCode,
            AccountStorageType::OnChain,
        );

        wallet.unwrap_or_else(|err| {
            panic!("{}", err);
        });
    }

    #[test]
    fn test_serialize_basic_wallet() {
        let pub_key = rpo_falcon512::PublicKey::new([ONE; 4]);
        let wallet = create_basic_wallet(
            [1; 32],
            AuthScheme::RpoFalcon512 { pub_key },
            AccountType::RegularAccountImmutableCode,
            AccountStorageType::OnChain,
        )
        .unwrap()
        .0;

        let bytes = wallet.to_bytes();
        let deserialized_wallet = Account::read_from_bytes(&bytes).unwrap();
        assert_eq!(wallet, deserialized_wallet);
    }
}
