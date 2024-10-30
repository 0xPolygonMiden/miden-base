use alloc::{sync::Arc, vec::Vec};
use std::string::ToString;

use assembly::{ast::Module, Assembler, LibraryPath};
use miden_crypto::dsa::rpo_falcon512::PublicKey;

use crate::{
    accounts::{AccountComponent, AssembledAccountComponent, StorageSlot},
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

pub const RPO_FALCON_AUTH_CODE: &str = "
    export.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
";

// BASIC WALLET ACCOUNT COMPONENT
// ================================================================================================

pub struct BasicWallet;

impl AccountComponent for BasicWallet {
    fn assemble_component(
        self,
        assembler: Assembler,
    ) -> Result<AssembledAccountComponent, AccountError> {
        AssembledAccountComponent::compile(BASIC_WALLET_CODE, assembler, vec![])
    }
}

// RPO FALCON 512 AUTH ACCOUNT COMPONENT
// ================================================================================================

pub struct RpoFalcon512 {
    public_key: PublicKey,
}

impl RpoFalcon512 {
    pub fn new(public_key: PublicKey) -> Self {
        Self { public_key }
    }
}

impl AccountComponent for RpoFalcon512 {
    fn assemble_component(
        self,
        assembler: Assembler,
    ) -> Result<AssembledAccountComponent, AccountError> {
        AssembledAccountComponent::compile(
            RPO_FALCON_AUTH_CODE,
            assembler,
            vec![StorageSlot::Value(self.public_key.into())],
        )
    }
}

// ACCOUNT MOCK COMPONENT
// ================================================================================================

/// Creates a mock [Library] which can be used to assemble programs and as a library to create a
/// mock [AccountCode] interface. Transaction and note scripts that make use of this interface
/// should be assembled with this.

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

impl AccountComponent for AccountMockComponent {
    fn assemble_component(
        self,
        assembler: Assembler,
    ) -> Result<AssembledAccountComponent, AccountError> {
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

        Ok(AssembledAccountComponent::new(library, self.storage_slots))
    }
}
