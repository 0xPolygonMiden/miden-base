use crate::assembler::assembler;
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AccountType, AccountVault},
    assembly::ModuleAst,
    crypto::{dsa::rpo_falcon512, merkle::MerkleStore},
    utils::vec,
    AccountError, Word, ZERO,
};

pub enum AuthScheme {
    RpoFalcon512 { pub_key: rpo_falcon512::PublicKey },
}

/// Creates a new faucet account with basic faucet interface, specified authentication scheme,
/// and provided meta data (token symbol, decimals, max supply).
///
/// The basic faucet interface exposes two procedures:
/// - `distribute`, which mints an assets and create a note for the provided recipient.
/// - `burn`, which burns the provided asset.
///
/// `distribute` requires authentication. The authentication procedure is defined by the specified
/// authentication scheme.
pub fn create_basic_faucet(
    init_seed: [u8; 32],
    meta_data: Word,
    auth_scheme: AuthScheme,
) -> Result<(Account, Word), AccountError> {
    let (_auth_scheme_prefix, _auth_scheme_procedure, storage_slot_0): (&str, &str, Word) =
        match auth_scheme {
            AuthScheme::RpoFalcon512 { pub_key } => {
                ("basic", "auth_tx_rpo_falcon512", pub_key.into())
            }
        };

    let account_code_src = include_str!("../../asm/faucets/basic.masm");

    // When we have more auth schemes, we will need to replace the code below with the code above.
    // match auth_scheme {
    //     AuthScheme::RpoFalcon512 { .. } => {
    //         account_code_src = include_str!("../../asm/faucets/basic.masm").to_string();
    //     }
    //     // this code is unreachable as of now, in the future there will be more auth schemes
    //     _ => {
    //         let auth_scheme_import: String = format!("use.miden::eoa::{}", auth_scheme_prefix);
    //         let auth_scheme_export: String = format!("export.{}.{})", auth_scheme_prefix, auth_scheme_procedure);
    //         account_code_src = account_code_src
    //             .replace("use.miden::eoa::basic", &auth_scheme_import)
    //             .replace("export.basic::auth_tx_rpo_falcon512", &auth_scheme_export);
    //     }
    // }

    let account_code_ast = ModuleAst::parse(account_code_src)
        .map_err(|e| AccountError::AccountCodeAssemblerError(e.into()))?;
    let account_assembler = assembler();
    let account_code = AccountCode::new(account_code_ast.clone(), &account_assembler)?;

    let account_storage =
        AccountStorage::new(vec![(0, storage_slot_0), (1, meta_data)], MerkleStore::new())?;
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
