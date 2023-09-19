use assembly::ast::ModuleAst;
use miden_objects::{
    mock::{assembler, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN},
    AccountCode, AccountError, AccountId,
};

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

pub fn str_to_accountcode(source: &str) -> Result<AccountCode, AccountError> {
    let assembler = assembler();
    let account_module_ast = ModuleAst::parse(source).unwrap();

    // There is a cyclic dependency among [AccountId] and [AccountCode], the id uses the coderoot
    // as part of its initial seed for commitment purposes, the code uses the id for error
    // reporting. Because the former is required for correctness and the later is only for error
    // messages, this generated an invalid [AccountId] to break the dependency cycle.
    let invalid_id: AccountId = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().unwrap();

    AccountCode::new(invalid_id, account_module_ast, &assembler)
}
