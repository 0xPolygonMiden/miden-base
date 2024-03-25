use miden_objects::{accounts::AccountCode, assembly::ModuleAst, AccountError};

use super::TransactionKernel;

mod account;
mod account_id;
mod account_storage;
mod error;
mod fungible_asset;
mod nonfungible_asset;
mod note;

// RE-EXPORTS
// ================================================================================================
pub use account::AccountBuilder;
pub use account_id::{accountid_build_details, AccountIdBuilder};
pub use account_storage::AccountStorageBuilder;
pub use error::AccountBuilderError;
pub use fungible_asset::FungibleAssetBuilder;
pub use nonfungible_asset::{NonFungibleAssetBuilder, NonFungibleAssetDetailsBuilder};
pub use note::NoteBuilder;

pub fn str_to_account_code(source: &str) -> Result<AccountCode, AccountError> {
    let assembler = TransactionKernel::assembler();
    let account_module_ast = ModuleAst::parse(source).unwrap();
    AccountCode::new(account_module_ast, &assembler)
}
