use alloc::string::ToString;

use miden_objects::{
    accounts::{
        Account, AccountCode, AccountComponent, AccountId, AccountStorage, AccountStorageMode,
        AccountType,
    },
    AccountError, Word,
};

use super::AuthScheme;
use crate::accounts::{auth::RpoFalcon512, components::basic_wallet_library};

// BASIC WALLET
// ================================================================================================

pub struct BasicWallet;

impl From<BasicWallet> for AccountComponent {
    fn from(_: BasicWallet) -> Self {
        AccountComponent::new(basic_wallet_library(), vec![])
    }
}

/// Creates a new account with basic wallet interface, the specified authentication scheme and the
/// account storage type. Basic wallets can be specified to have either mutable or immutable code.
///
/// The basic wallet interface exposes three procedures:
/// - `receive_asset`, which can be used to add an asset to the account.
/// - `create_note`, which can be used to create a new note without any assets attached to it.
/// - `move_asset_to_note`, which can be used to remove the specified asset from the account and add
///   it to the output note with the specified index.
///
/// All methods require authentication. The authentication procedure is defined by the specified
/// authentication scheme. Public key information for the scheme is stored in the account storage
/// at slot 0.
pub fn create_basic_wallet(
    init_seed: [u8; 32],
    auth_scheme: AuthScheme,
    account_type: AccountType,
    account_storage_mode: AccountStorageMode,
) -> Result<(Account, Word), AccountError> {
    if matches!(account_type, AccountType::FungibleFaucet | AccountType::NonFungibleFaucet) {
        return Err(AccountError::AccountIdInvalidFieldElement(
            "Basic wallet accounts cannot have a faucet account type".to_string(),
        ));
    }

    let auth_component = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => RpoFalcon512::new(pub_key).into(),
    };

    let components = [auth_component, BasicWallet.into()];

    let account_code = AccountCode::from_components(&components)?;
    let account_storage = AccountStorage::from_components(&components)?;

    let account_seed = AccountId::get_account_seed(
        init_seed,
        account_type,
        account_storage_mode,
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

    use super::{create_basic_wallet, Account, AccountStorageMode, AccountType, AuthScheme};

    #[test]
    fn test_create_basic_wallet() {
        let pub_key = rpo_falcon512::PublicKey::new([ONE; 4]);
        let wallet = create_basic_wallet(
            [1; 32],
            AuthScheme::RpoFalcon512 { pub_key },
            AccountType::RegularAccountImmutableCode,
            AccountStorageMode::Public,
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
            AccountStorageMode::Public,
        )
        .unwrap()
        .0;

        let bytes = wallet.to_bytes();
        let deserialized_wallet = Account::read_from_bytes(&bytes).unwrap();
        assert_eq!(wallet, deserialized_wallet);
    }
}
