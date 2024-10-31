use alloc::string::ToString;

use miden_objects::{
    accounts::{
        Account, AccountCode, AccountComponent, AccountComponentType, AccountId, AccountStorage,
        AccountStorageMode, AccountType, StorageSlot,
    },
    assets::TokenSymbol,
    AccountError, Felt, FieldElement, Word,
};

use super::AuthScheme;
use crate::accounts::{auth::RpoFalcon512, components::basic_fungible_faucet_library};

// BASIC FUNGIBLE FAUCET ACCOUNT COMPONENT
// ================================================================================================

pub struct BasicFungibleFaucet {
    symbol: TokenSymbol,
    decimals: u8,
    max_supply: Felt,
}

impl BasicFungibleFaucet {
    pub fn new(symbol: TokenSymbol, decimals: u8, max_supply: Felt) -> Result<Self, AccountError> {
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

        Ok(Self { symbol, decimals, max_supply })
    }
}

impl From<BasicFungibleFaucet> for AccountComponent {
    fn from(faucet: BasicFungibleFaucet) -> Self {
        // Note: data is stored as [a0, a1, a2, a3] but loaded onto the stack as
        // [a3, a2, a1, a0, ...]
        let metadata =
            [faucet.max_supply, Felt::from(faucet.decimals), faucet.symbol.into(), Felt::ZERO];

        AccountComponent::new(basic_fungible_faucet_library(), vec![StorageSlot::Value(metadata)])
            .with_type(AccountComponentType::Faucet)
    }
}

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
    account_storage_mode: AccountStorageMode,
    auth_scheme: AuthScheme,
) -> Result<(Account, Word), AccountError> {
    // Atm we only have RpoFalcon512 as authentication scheme and this is also the default in the
    // faucet contract.
    let auth_component = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => RpoFalcon512::new(pub_key).into(),
    };
    let faucet_component = BasicFungibleFaucet::new(symbol, decimals, max_supply)?.into();
    let components = [auth_component, faucet_component];

    let account_code = AccountCode::from_components(&components)?;
    let account_storage = AccountStorage::from_components(&components)?;

    let account_seed = AccountId::get_account_seed(
        init_seed,
        AccountType::FungibleFaucet,
        account_storage_mode,
        account_code.commitment(),
        account_storage.commitment(),
    )?;

    let account = Account::new(account_seed, account_code, account_storage)?;

    Ok((account, account_seed))
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use miden_objects::{crypto::dsa::rpo_falcon512, FieldElement, ONE};

    use super::{create_basic_fungible_faucet, AccountStorageMode, AuthScheme, Felt, TokenSymbol};

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
        let storage_mode = AccountStorageMode::Private;

        let (faucet_account, _) = create_basic_fungible_faucet(
            init_seed,
            token_symbol,
            decimals,
            max_supply,
            storage_mode,
            auth_scheme,
        )
        .unwrap();

        // Check that faucet metadata was initialized to the given values.
        // The RpoFalcon512 component is added first and the faucet component second, so its
        // assigned storage slot will be 2.
        assert_eq!(
            faucet_account.storage().get_item(2).unwrap(),
            [Felt::new(123), Felt::new(2), token_symbol.into(), Felt::ZERO].into()
        );

        assert!(faucet_account.is_faucet());
    }
}
