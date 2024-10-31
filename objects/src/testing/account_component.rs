use alloc::{sync::Arc, vec::Vec};
use std::string::ToString;

use assembly::{ast::Module, Assembler, Library, LibraryPath};

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

/// Creates a mock [`Library`] which can be used to assemble programs and as a library to create a
/// mock [`AccountCode`](crate::accounts::AccountCode) interface. Transaction and note scripts that
/// make use of this interface should be assembled with this.
///
/// This component supports all [`AccountType`](crate::accounts::AccountType)s for testing purposes.
pub struct AccountMockComponent {
    library: Library,
    storage_slots: Vec<StorageSlot>,
}

impl AccountMockComponent {
    fn new(assembler: Assembler, storage_slots: Vec<StorageSlot>) -> Result<Self, AccountError> {
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

        Ok(Self { library, storage_slots })
    }

    pub fn new_with_empty_slots(assembler: Assembler) -> Result<Self, AccountError> {
        Self::new(assembler, vec![])
    }

    pub fn new_with_slots(
        assembler: Assembler,
        storage_slots: Vec<StorageSlot>,
    ) -> Result<Self, AccountError> {
        Self::new(assembler, storage_slots)
    }
}

impl From<AccountMockComponent> for Library {
    fn from(mock_component: AccountMockComponent) -> Self {
        mock_component.library
    }
}

impl From<AccountMockComponent> for AccountComponent {
    fn from(mock_component: AccountMockComponent) -> Self {
        AccountComponent::new(mock_component.library, mock_component.storage_slots)
            .with_supports_all_types()
    }
}
