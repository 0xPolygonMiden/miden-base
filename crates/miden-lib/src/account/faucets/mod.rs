use miden_objects::{
    AccountError, Felt, FieldElement, Word,
    account::{
        Account, AccountBuilder, AccountComponent, AccountIdAnchor, AccountStorageMode,
        AccountType, StorageSlot,
    },
    asset::{FungibleAsset, TokenSymbol},
};

use super::AuthScheme;
use crate::account::{auth::RpoFalcon512, components::basic_fungible_faucet_library};

// BASIC FUNGIBLE FAUCET ACCOUNT COMPONENT
// ================================================================================================

/// An [`AccountComponent`] implementing a basic fungible faucet.
///
/// It reexports the procedures from `miden::contracts::faucets::basic_fungible`. When linking
/// against this component, the `miden` library (i.e. [`MidenLib`](crate::MidenLib)) must be
/// available to the assembler which is the case when using
/// [`TransactionKernel::assembler()`][kasm]. The procedures of this component are:
/// - `distribute`, which mints an assets and create a note for the provided recipient.
/// - `burn`, which burns the provided asset.
///
/// `distribute` requires authentication while `burn` does not require authentication and can be
/// called by anyone. Thus, this component must be combined with a component providing
/// authentication.
///
/// This component supports accounts of type [`AccountType::FungibleFaucet`].
///
/// [kasm]: crate::transaction::TransactionKernel::assembler
pub struct BasicFungibleFaucet {
    symbol: TokenSymbol,
    decimals: u8,
    max_supply: Felt,
}

impl BasicFungibleFaucet {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// The maximum number of decimals supported by the component.
    pub const MAX_DECIMALS: u8 = 12;

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`BasicFungibleFaucet`] component from the given pieces of metadata.
    pub fn new(symbol: TokenSymbol, decimals: u8, max_supply: Felt) -> Result<Self, AccountError> {
        // First check that the metadata is valid.
        if decimals > Self::MAX_DECIMALS {
            return Err(AccountError::FungibleFaucetTooManyDecimals {
                actual: decimals,
                max: Self::MAX_DECIMALS,
            });
        } else if max_supply.as_int() > FungibleAsset::MAX_AMOUNT {
            return Err(AccountError::FungibleFaucetMaxSupplyTooLarge {
                actual: max_supply.as_int(),
                max: FungibleAsset::MAX_AMOUNT,
            });
        }

        Ok(Self { symbol, decimals, max_supply })
    }
}

impl From<BasicFungibleFaucet> for AccountComponent {
    fn from(faucet: BasicFungibleFaucet) -> Self {
        // Note: data is stored as [a0, a1, a2, a3] but loaded onto the stack as
        // [a3, a2, a1, a0, ...]
        let metadata =
            [faucet.max_supply, Felt::from(faucet.decimals), faucet.symbol.into(), Felt::ZERO];

        AccountComponent::new(basic_fungible_faucet_library(), vec![StorageSlot::Value(metadata)])
            .expect("basic fungible faucet component should satisfy the requirements of a valid account component")
            .with_supported_type(AccountType::FungibleFaucet)
    }
}

// FUNGIBLE FAUCET
// ================================================================================================

/// Creates a new faucet account with basic fungible faucet interface,
/// account storage type, specified authentication scheme, and provided meta data (token symbol,
/// decimals, max supply).
///
/// The basic faucet interface exposes two procedures:
/// - `distribute`, which mints an assets and create a note for the provided recipient.
/// - `burn`, which burns the provided asset.
///
/// `distribute` requires authentication. The authentication procedure is defined by the specified
/// authentication scheme. `burn` does not require authentication and can be called by anyone.
///
/// The storage layout of the faucet account is:
/// - Slot 0: Reserved slot for faucets.
/// - Slot 1: Public Key of the authentication component.
/// - Slot 2: Token metadata of the faucet.
pub fn create_basic_fungible_faucet(
    init_seed: [u8; 32],
    id_anchor: AccountIdAnchor,
    symbol: TokenSymbol,
    decimals: u8,
    max_supply: Felt,
    account_storage_mode: AccountStorageMode,
    auth_scheme: AuthScheme,
) -> Result<(Account, Word), AccountError> {
    // Atm we only have RpoFalcon512 as authentication scheme and this is also the default in the
    // faucet contract.
    let auth_component: RpoFalcon512 = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => RpoFalcon512::new(pub_key),
    };

    let (account, account_seed) = AccountBuilder::new(init_seed)
        .anchor(id_anchor)
        .account_type(AccountType::FungibleFaucet)
        .storage_mode(account_storage_mode)
        .with_component(auth_component)
        .with_component(BasicFungibleFaucet::new(symbol, decimals, max_supply)?)
        .build()?;

    Ok((account, account_seed))
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use miden_objects::{
        FieldElement, ONE, block::BlockHeader, crypto::dsa::rpo_falcon512, digest,
    };
    use vm_processor::Word;

    use super::{AccountStorageMode, AuthScheme, Felt, TokenSymbol, create_basic_fungible_faucet};

    #[test]
    fn faucet_contract_creation() {
        let pub_key = rpo_falcon512::PublicKey::new([ONE; 4]);
        let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key };

        // we need to use an initial seed to create the wallet account
        let init_seed: [u8; 32] = [
            90, 110, 209, 94, 84, 105, 250, 242, 223, 203, 216, 124, 22, 159, 14, 132, 215, 85,
            183, 204, 149, 90, 166, 68, 100, 73, 106, 168, 125, 237, 138, 16,
        ];

        let max_supply = Felt::new(123);
        let token_symbol_string = "POL";
        let token_symbol = TokenSymbol::try_from(token_symbol_string).unwrap();
        let decimals = 2u8;
        let storage_mode = AccountStorageMode::Private;

        let anchor_block_header_mock = BlockHeader::mock(
            0,
            Some(digest!("0xaa")),
            Some(digest!("0xbb")),
            &[],
            digest!("0xcc"),
        );

        let (faucet_account, _) = create_basic_fungible_faucet(
            init_seed,
            (&anchor_block_header_mock).try_into().unwrap(),
            token_symbol,
            decimals,
            max_supply,
            storage_mode,
            auth_scheme,
        )
        .unwrap();

        // The reserved faucet slot should be initialized to an empty word.
        assert_eq!(faucet_account.storage().get_item(0).unwrap(), Word::default().into());

        // The falcon auth component is added first so its assigned storage slot for the public key
        // will be 1.
        assert_eq!(faucet_account.storage().get_item(1).unwrap(), Word::from(pub_key).into());

        // Check that faucet metadata was initialized to the given values. The faucet component is
        // added second, so its assigned storage slot for the metadata will be 2.
        assert_eq!(
            faucet_account.storage().get_item(2).unwrap(),
            [Felt::new(123), Felt::new(2), token_symbol.into(), Felt::ZERO].into()
        );

        assert!(faucet_account.is_faucet());
    }
}
