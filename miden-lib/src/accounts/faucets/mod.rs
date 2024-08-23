use alloc::{collections::BTreeMap, string::ToString};

use miden_objects::{
    accounts::{
        Account, AccountCode, AccountId, AccountStorage, AccountStorageType, AccountType, SlotItem,
    },
    assets::TokenSymbol,
    AccountError, Felt, Word, ZERO,
};

use super::{AuthScheme, TransactionKernel};

// FUNGIBLE FAUCET
// ================================================================================================

const MAX_MAX_SUPPLY: u64 = (1 << 63) - 1;
const MAX_DECIMALS: u8 = 12;

/// Creates a new faucet account with basic fungible faucet interface,
/// account storage type, specified authentication scheme, and provided meta data (token symbol,
/// decimals, max supply).
///
/// The basic faucet interface exposes two procedures:
/// - `distribute`, which mints an assets and create a note for the provided recipient.
/// - `burn`, which burns the provided asset.
///
/// `distribute` requires authentication. The authentication procedure is defined by the specified
/// authentication scheme. `burn` does not require authentication and can be called by anyone.
///
/// Public key information for the scheme is stored in the account storage at slot 0. The token
/// metadata is stored in the account storage at slot 1.
pub fn create_basic_fungible_faucet(
    init_seed: [u8; 32],
    symbol: TokenSymbol,
    decimals: u8,
    max_supply: Felt,
    account_storage_type: AccountStorageType,
    auth_scheme: AuthScheme,
) -> Result<(Account, Word), AccountError> {
    // Atm we only have RpoFalcon512 as authentication scheme and this is also the default in the
    // faucet contract, so we can just use the public key as storage slot 0.

    let (auth_scheme_procedure, auth_data): (&str, Word) = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => ("auth_tx_rpo_falcon512", pub_key.into()),
    };

    let source_code = format!(
        "
        export.::miden::contracts::faucets::basic_fungible::distribute
        export.::miden::contracts::faucets::basic_fungible::burn
        export.::miden::contracts::auth::basic::{auth_scheme_procedure}
    "
    );

    let assembler = TransactionKernel::assembler();
    let account_code = AccountCode::compile(source_code, assembler)?;

    // First check that the metadata is valid.
    if decimals > MAX_DECIMALS {
        return Err(AccountError::FungibleFaucetInvalidMetadata(
            "Decimals must be less than 13".to_string(),
        ));
    } else if max_supply.as_int() > MAX_MAX_SUPPLY {
        return Err(AccountError::FungibleFaucetInvalidMetadata(
            "Max supply must be < 2^63".to_string(),
        ));
    }

    // Note: data is stored as [a0, a1, a2, a3] but loaded onto the stack as [a3, a2, a1, a0, ...]
    let metadata = [max_supply, Felt::from(decimals), symbol.into(), ZERO];

    // We store the authentication data and the token metadata in the account storage:
    // - slot 0: authentication data
    // - slot 1: token metadata as [max_supply, decimals, token_symbol, 0]
    let account_storage = AccountStorage::new(
        vec![SlotItem::new_value(0, 0, auth_data), SlotItem::new_value(1, 0, metadata)],
        BTreeMap::new(),
    )?;

    let account_seed = AccountId::get_account_seed(
        init_seed,
        AccountType::FungibleFaucet,
        account_storage_type,
        account_code.commitment(),
        account_storage.root(),
    )?;

    Ok((Account::new(account_seed, account_code, account_storage)?, account_seed))
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use miden_objects::{crypto::dsa::rpo_falcon512, ONE};

    use super::{
        create_basic_fungible_faucet, AccountStorageType, AuthScheme, Felt, TokenSymbol, ZERO,
    };

    #[test]
    fn faucet_contract_creation() {
        let pub_key = rpo_falcon512::PublicKey::new([ONE; 4]);
        let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key };

        // we need to use an initial seed to create the wallet account
        let init_seed: [u8; 32] = [
            90, 110, 209, 94, 84, 105, 250, 242, 223, 203, 216, 124, 22, 159, 14, 132, 215, 85,
            183, 204, 149, 90, 166, 68, 100, 73, 106, 168, 125, 237, 138, 16,
        ];

        let max_supply = Felt::new(123);
        let token_symbol_string = "POL";
        let token_symbol = TokenSymbol::try_from(token_symbol_string).unwrap();
        let decimals = 2u8;
        let storage_type = AccountStorageType::OffChain;

        let (faucet_account, _) = create_basic_fungible_faucet(
            init_seed,
            token_symbol,
            decimals,
            max_supply,
            storage_type,
            auth_scheme,
        )
        .unwrap();

        // check that max_supply (slot 1) is 123
        assert_eq!(
            faucet_account.storage().get_item(1),
            [Felt::new(123), Felt::new(2), token_symbol.into(), ZERO].into()
        );

        assert!(faucet_account.is_faucet());
    }
}
