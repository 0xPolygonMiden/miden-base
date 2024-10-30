use alloc::{sync::Arc, vec::Vec};
use std::string::ToString;

use assembly::{ast::Module, Assembler, LibraryPath};
use miden_crypto::dsa::rpo_falcon512::PublicKey;
use vm_core::{Felt, FieldElement};

use crate::{
    accounts::{AccountComponent, AccountComponentType, AssembledAccountComponent, StorageSlot},
    assets::TokenSymbol,
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

const BASIC_FUNGIBLE_FAUCET_CODE: &str = "
    export.::miden::contracts::faucets::basic_fungible::distribute
    export.::miden::contracts::faucets::basic_fungible::burn
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

// BASIC FUNGIBLE FAUCET ACCOUNT COMPONENT
// ================================================================================================

pub struct BasicFungibleFaucet {
    symbol: TokenSymbol,
    decimals: u8,
    max_supply: Felt,
}

impl BasicFungibleFaucet {
    pub fn new(symbol: TokenSymbol, decimals: u8, max_supply: Felt) -> Self {
        Self { symbol, decimals, max_supply }
    }
}

impl AccountComponent for BasicFungibleFaucet {
    fn assemble_component(
        self,
        assembler: Assembler,
    ) -> Result<AssembledAccountComponent, AccountError> {
        // Note: data is stored as [a0, a1, a2, a3] but loaded onto the stack as
        // [a3, a2, a1, a0, ...]
        let metadata = [self.max_supply, Felt::from(self.decimals), self.symbol.into(), Felt::ZERO];

        AssembledAccountComponent::compile(
            BASIC_FUNGIBLE_FAUCET_CODE,
            assembler,
            vec![StorageSlot::Value(metadata)],
        )
        .map(|component| component.with_type(AccountComponentType::Faucet))
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
