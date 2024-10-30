use alloc::{sync::Arc, vec::Vec};

use assembly::{ast::Module, Assembler, LibraryPath};
use miden_crypto::dsa::rpo_falcon512::PublicKey;
use vm_core::{Felt, FieldElement};

use crate::{
    accounts::{AccountComponent, AccountComponentType, StorageSlot},
    assets::TokenSymbol,
    testing::account_code::MOCK_ACCOUNT_CODE,
};

// ACCOUNT ASSEMBLY CODE
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

pub trait IntoAccountComponent {
    fn into_component(self, assembler: Assembler) -> AccountComponent;
}

impl IntoAccountComponent for AccountComponent {
    fn into_component(self, _: Assembler) -> AccountComponent {
        self
    }
}

// BASIC WALLET ACCOUNT COMPONENT
// ================================================================================================

pub struct BasicWallet;

impl IntoAccountComponent for BasicWallet {
    fn into_component(self, assembler: Assembler) -> AccountComponent {
        AccountComponent::compile(BASIC_WALLET_CODE, assembler, vec![]).unwrap()
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

impl IntoAccountComponent for RpoFalcon512 {
    fn into_component(self, assembler: Assembler) -> AccountComponent {
        AccountComponent::compile(
            RPO_FALCON_AUTH_CODE,
            assembler,
            vec![StorageSlot::Value(self.public_key.into())],
        )
        .unwrap()
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

impl IntoAccountComponent for BasicFungibleFaucet {
    fn into_component(self, assembler: Assembler) -> AccountComponent {
        // Note: data is stored as [a0, a1, a2, a3] but loaded onto the stack as
        // [a3, a2, a1, a0, ...]
        let metadata = [self.max_supply, Felt::from(self.decimals), self.symbol.into(), Felt::ZERO];

        AccountComponent::compile(
            BASIC_FUNGIBLE_FAUCET_CODE,
            assembler,
            vec![StorageSlot::Value(metadata)],
        )
        .unwrap()
        .with_type(AccountComponentType::Faucet)
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

impl IntoAccountComponent for AccountMockComponent {
    fn into_component(self, assembler: Assembler) -> AccountComponent {
        let source_manager = Arc::new(assembly::DefaultSourceManager::default());
        let module = Module::parser(assembly::ast::ModuleKind::Library)
            .parse_str(
                LibraryPath::new("test::account").unwrap(),
                MOCK_ACCOUNT_CODE,
                &source_manager,
            )
            .unwrap();

        let library = assembler.assemble_library(&[*module]).unwrap();

        AccountComponent::new(library, self.storage_slots)
    }
}