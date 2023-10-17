use crate::{assembler::assembler, auth::AuthScheme};
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AccountType, AccountVault},
    assembly::ModuleAst,
    crypto::merkle::MerkleStore,
    utils::{
        string::{String, ToString},
        vec,
    },
    AccountError, Felt, StarkField, Word, ZERO,
};

/// Creates a new faucet account with basic faucet interface, specified authentication scheme,
/// and provided meta data (token symbol, decimals, max supply).
///
/// The basic faucet interface exposes two procedures:
/// - `distribute`, which mints an assets and create a note for the provided recipient.
/// - `burn`, which burns the provided asset.
///
/// `distribute` requires authentication. The authentication procedure is defined by the specified
/// authentication scheme.
/// `burn` does not require authentication and can be called by anyone.
/// Public key information for the scheme is stored in the account storage at slot 0.
/// The token metadata is stored in the account storage at slot 1.
pub fn create_basic_faucet(
    init_seed: [u8; 32],
    symbol: String,
    decimals: u8,
    max_supply: Felt,
    auth_scheme: AuthScheme,
) -> Result<(Account, Word), AccountError> {
    // Atm we onlt have RpoFalcon512 as authentication scheme and this is also the default in the
    // faucet contract, so we can just use the public key as storage slot 0.
    let auth_data: Word = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => pub_key.into(),
    };

    let account_code_src = include_str!("../../asm/faucets/basic.masm");
    let account_code_ast = ModuleAst::parse(account_code_src)
        .map_err(|e| AccountError::AccountCodeAssemblerError(e.into()))?;
    let account_assembler = assembler();
    let account_code = AccountCode::new(account_code_ast.clone(), &account_assembler)?;

    // First check that the metadata is valid.
    if decimals > 18 {
        return Err(AccountError::FungibleFaucetInvalidMetadata(
            "Decimals must be less than 19".to_string(),
        ));
    } else if symbol.len() > 3 {
        return Err(AccountError::FungibleFaucetInvalidMetadata(
            "Token Symbol must have exactly three characters".to_string(),
        ));
    } else if max_supply.as_int() == 0 {
        return Err(AccountError::FungibleFaucetInvalidMetadata(
            "Max supply must be > 0".to_string(),
        ));
    }

    // Note: order is reversed here to match, the faucet contract assumes to get max_supply by
    // mem_loadw drop drop drop
    let metadata =
        [max_supply, ZERO, Felt::from(decimals), encode_symbol_to_felt(&symbol).unwrap()];

    // We store the authentication data and the token metadata in the account storage:
    // - slot 0: authentication data
    // - slot 1: token metadata as [token_symbol, decimals, 0, max_supply]
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

/// Util to encode and decode the token symbol as a Felt. We allow any three character string as
/// token symbol, e.g., AAA = 0, ...
pub fn encode_symbol_to_felt(s: &str) -> Result<Felt, &'static str> {
    let mut encoded_value = 0;
    for (i, char) in s.chars().enumerate() {
        encoded_value += (char as u64 - 'A' as u64) * 26u64.pow((2 - i) as u32);
    }

    Ok(Felt::new(encoded_value))
}

pub fn decode_felt_to_symbol(encoded_felt: Felt) -> Result<String, &'static str> {
    let encoded_value = encoded_felt.as_int();
    if encoded_value >= 26u64.pow(3) {
        return Err("Encoded value is out of range");
    }

    let mut decoded_string = String::new();
    let mut remaining_value = encoded_value;

    for i in (0..3).rev() {
        let quotient = remaining_value / 26u64.pow(i as u32);
        remaining_value %= 26u64.pow(i as u32);
        let char = ((quotient + 'A' as u64) as u8) as char;
        decoded_string.push(char);
    }

    Ok(decoded_string)
}
