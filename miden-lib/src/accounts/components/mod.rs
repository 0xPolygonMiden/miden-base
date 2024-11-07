use miden_objects::{
    assembly::Library,
    utils::{sync::LazyLock, Deserializable},
};

// Initialize the Basic Wallet library only once.
static BASIC_WALLET_LIBRARY: LazyLock<Library> = LazyLock::new(|| {
    let bytes =
        include_bytes!(concat!(env!("OUT_DIR"), "/assets/account_components/basic_wallet.masl"));
    Library::read_from_bytes(bytes).expect("Shipped Basic Wallet library is well-formed")
});

// Initialize the Rpo Falcon 512 library only once.
static RPO_FALCON_512_LIBRARY: LazyLock<Library> = LazyLock::new(|| {
    let bytes =
        include_bytes!(concat!(env!("OUT_DIR"), "/assets/account_components/rpo_falcon_512.masl"));
    Library::read_from_bytes(bytes).expect("Shipped Rpo Falcon 512 library is well-formed")
});

// Initialize the Basic Fungible Faucet library only once.
static BASIC_FUNGIBLE_FAUCET_LIBRARY: LazyLock<Library> = LazyLock::new(|| {
    let bytes = include_bytes!(concat!(
        env!("OUT_DIR"),
        "/assets/account_components/basic_fungible_faucet.masl"
    ));
    Library::read_from_bytes(bytes).expect("Shipped Basic Fungible Faucet library is well-formed")
});

/// Returns a reference to the Basic Wallet Library.
pub fn basic_wallet_library() -> &'static Library {
    BASIC_WALLET_LIBRARY.as_ref()
}

/// Returns a reference to the Rpo Falcon 512 Library.
pub fn rpo_falcon_512_library() -> &'static Library {
    RPO_FALCON_512_LIBRARY.as_ref()
}

/// Returns a reference to the Basic Fungible Faucet Library.
pub fn basic_fungible_faucet_library() -> &'static Library {
    BASIC_FUNGIBLE_FAUCET_LIBRARY.as_ref()
}

#[cfg(test)]
mod tests {
    use miden_objects::assembly::Assembler;

    use super::*;

    /// Test that the account component libraries can be used to link against from other MASM code
    /// and in particular that they are all available under the same library namespace
    /// ("account_components").
    #[test]
    fn test_account_component_libraries_can_be_linked() {
        let source = r#"
        use.account_components::basic_wallet
        use.account_components::rpo_falcon_512
        use.account_components::basic_fungible_faucet

        begin
          call.basic_wallet::receive_asset
          call.rpo_falcon_512::auth_tx_rpo_falcon512
          call.basic_fungible_faucet::distribute
        end
        "#;

        Assembler::default()
            .with_library(basic_wallet_library())
            .unwrap()
            .with_library(rpo_falcon_512_library())
            .unwrap()
            .with_library(basic_fungible_faucet_library())
            .unwrap()
            .assemble_program(source)
            .expect("we should be able to link against the account component libraries");
    }
}
