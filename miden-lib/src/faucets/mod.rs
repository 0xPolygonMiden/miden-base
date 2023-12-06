use super::Library;
use crate::{assembler::assembler, auth::AuthScheme};
use assembly::LibraryPath;
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AccountType, AccountVault},
    assets::TokenSymbol,
    crypto::merkle::MerkleStore,
    utils::{string::ToString, vec},
    AccountError, Felt, StarkField, Word, ZERO,
};

const MAX_MAX_SUPPLY: u64 = (1 << 63) - 1;
const MAX_DECIMALS: u8 = 12;

/// Creates a new faucet account with basic fungible faucet interface,
/// specified authentication scheme, and provided meta data (token symbol, decimals, max supply).
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
    auth_scheme: AuthScheme,
) -> Result<(Account, Word), AccountError> {
    // Atm we only have RpoFalcon512 as authentication scheme and this is also the default in the
    // faucet contract, so we can just use the public key as storage slot 0.
    // TODO: consider using a trait when we have more auth schemes.
    let auth_data: Word = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => pub_key.into(),
    };

    let miden = super::MidenLib::default();
    let path = "miden::faucets::basic_fungible";
    let faucet_code_ast = miden
        .get_module_ast(&LibraryPath::new(path).unwrap())
        .expect("Getting module AST failed");

    let account_assembler = assembler();
    let account_code = AccountCode::new(faucet_code_ast.clone(), &account_assembler)?;

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
    let account_storage =
        AccountStorage::new(vec![(0, auth_data), (1, metadata)], MerkleStore::new())?;
    let account_vault = AccountVault::new(&[])?;

    let account_seed = AccountId::get_account_seed(
        init_seed,
        AccountType::FungibleFaucet,
        false,
        account_code.root(),
        account_storage.root(),
    )?;
    let account_id = AccountId::new(account_seed, account_code.root(), account_storage.root())?;
    Ok((
        Account::new(account_id, account_vault, account_storage, account_code, ZERO),
        account_seed,
    ))
}
