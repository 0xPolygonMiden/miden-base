use alloc::{sync::Arc, vec::Vec};
use std::string::ToString;

use assembly::{ast::Module, Assembler, LibraryPath};

use crate::{
    accounts::{AccountComponent, StorageSlot},
    testing::account_code::MOCK_ACCOUNT_CODE,
    AccountError,
};

// ACCOUNT COMPONENT ASSEMBLY CODE
// ================================================================================================

pub const BASIC_WALLET_CODE: &str = "
    export.::miden::contracts::wallets::basic::receive_asset
    export.::miden::contracts::wallets::basic::create_note
    export.::miden::contracts::wallets::basic::move_asset_to_note
";

// ACCOUNT MOCK COMPONENT
// ================================================================================================

/// Creates a mock account component which can be used as a library to create a mock
/// [`AccountCode`](crate::accounts::AccountCode) interface. Transaction and note scripts that make
/// use of this interface should be assembled with this.
pub struct AccountMockComponent {
    storage_slots: Vec<StorageSlot>,
}

impl AccountMockComponent {
    pub fn with_empty_slots() -> Self {
        Self { storage_slots: vec![] }
    }

    pub fn with_slots(storage_slots: Vec<StorageSlot>) -> Self {
        Self { storage_slots }
    }
}

impl AccountMockComponent {
    pub fn assemble_component(
        self,
        assembler: Assembler,
    ) -> Result<AccountComponent, AccountError> {
        let source_manager = Arc::new(assembly::DefaultSourceManager::default());
        let module = Module::parser(assembly::ast::ModuleKind::Library)
            .parse_str(
                LibraryPath::new("test::account").unwrap(),
                MOCK_ACCOUNT_CODE,
                &source_manager,
            )
            .map_err(|report| AccountError::AccountCodeAssemblyError(report.to_string()))?;

        let library = assembler
            .assemble_library(&[*module])
            .map_err(|report| AccountError::AccountCodeAssemblyError(report.to_string()))?;

        Ok(AccountComponent::new(library, self.storage_slots))
    }
}
