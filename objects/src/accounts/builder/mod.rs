use alloc::vec::Vec;
use core::fmt::Display;

use crate::{
    accounts::{Account, AccountComponent, AccountId, AccountStorageMode, AccountType},
    assets::{Asset, AssetVault},
    AccountError, AssetVaultError, Felt, Word, ZERO,
};

/// A convenient builder for an [`Account`] allowing for safe construction of an account by
/// combining multiple [`AccountComponent`]s.
///
/// By default, the builder is initialized with:
/// - The `nonce` set to [`ZERO`], i.e. the nonce of a new account.
/// - The `account_type` set to [`AccountType::RegularAccountUpdatableCode`].
/// - The `storage_mode` set to [`AccountStorageMode::Private`].
///
/// The methods that are required to be called are:
///
/// - [`AccountBuilder::init_seed`],
/// - [`AccountBuilder::with_component`], which must be called at least once.
#[derive(Debug, Clone)]
pub struct AccountBuilder {
    assets: Vec<Asset>,
    components: Vec<AccountComponent>,
    nonce: Felt,
    account_type: AccountType,
    storage_mode: AccountStorageMode,
    init_seed: Option<[u8; 32]>,
}

impl AccountBuilder {
    /// Creates a new builder for a single account.
    pub fn new() -> Self {
        Self {
            assets: vec![],
            components: vec![],
            nonce: ZERO,
            init_seed: None,
            account_type: AccountType::RegularAccountUpdatableCode,
            storage_mode: AccountStorageMode::Private,
        }
    }

    /// Sets the initial seed from which the grind for an [`AccountId`] will start. This initial
    /// seed should come from a cryptographic random number generator.
    ///
    ///  This method **must** be called.
    pub fn init_seed(mut self, init_seed: [u8; 32]) -> Self {
        self.init_seed = Some(init_seed);
        self
    }

    /// Sets the type of the account.
    pub fn account_type(mut self, account_type: AccountType) -> Self {
        self.account_type = account_type;
        self
    }

    /// Sets the storage mode of the account.
    pub fn storage_mode(mut self, storage_mode: AccountStorageMode) -> Self {
        self.storage_mode = storage_mode;
        self
    }

    /// Sets the nonce of the account. This method is optional.
    ///
    /// If unset, the nonce will default to [`ZERO`].
    pub fn nonce(mut self, nonce: Felt) -> Self {
        self.nonce = nonce;
        self
    }

    /// Adds the asset to the account's [`AssetVault`]. This method is optional.
    pub fn with_asset(mut self, asset: Asset) -> Self {
        self.assets.push(asset);
        self
    }

    /// Adds all the assets to the account's [`AssetVault`]. This method is optional.
    pub fn with_assets<I: IntoIterator<Item = Asset>>(mut self, assets: I) -> Self {
        self.assets.extend(assets);
        self
    }

    /// Adds an [`AccountComponent`] to the builder. This method can be called multiple times and
    /// **must be called at least once** since an account must export at least one procedure.
    ///
    /// All components will be merged to form the final code and storage of the built account.
    pub fn with_component(mut self, account_component: impl Into<AccountComponent>) -> Self {
        self.components.push(account_component.into());
        self
    }

    /// Builds an [`Account`] out of the configured builder.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The init seed is not set.
    /// - If a duplicate assets was added to the builder.
    /// - Any of the components does not support the set account type.
    /// - The number of procedures in all merged components is 0 or exceeds
    ///   [`AccountCode::MAX_NUM_PROCEDURES`](crate::accounts::AccountCode::MAX_NUM_PROCEDURES).
    /// - Two or more libraries export a procedure with the same MAST root.
    /// - The number of [`StorageSlot`](crate::accounts::StorageSlot)s of all components exceeds
    ///   255.
    /// - [`MastForest::merge`](vm_processor::MastForest::merge) fails on the given components.
    pub fn build(self) -> Result<(Account, Word), AccountBuildError> {
        let init_seed = self.init_seed.ok_or(AccountBuildError::AccountInitSeedNotSet)?;

        let vault = AssetVault::new(&self.assets).map_err(AccountBuildError::AssetVaultError)?;

        let (code, storage) =
            Account::initialize_from_components(self.account_type, &self.components)
                .map_err(AccountBuildError::ComponentInitializationError)?;

        let code_commitment = code.commitment();
        let storage_commitment = storage.commitment();

        let seed = AccountId::get_account_seed(
            init_seed,
            self.account_type,
            self.storage_mode,
            code_commitment,
            storage_commitment,
        )
        .map_err(AccountBuildError::AccountSeedGenerationFailure)?;

        let account_id = AccountId::new(seed, code_commitment, storage_commitment)
            .expect("get_account_seed should provide a suitable seed");

        debug_assert_eq!(account_id.account_type(), self.account_type);
        debug_assert_eq!(account_id.storage_mode(), self.storage_mode);

        let account = Account::from_parts(account_id, vault, storage, code, self.nonce);
        Ok((account, seed))
    }
}

impl Default for AccountBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountBuildError {
    AccountInitSeedNotSet,
    AccountSeedGenerationFailure(AccountError),
    ComponentInitializationError(AccountError),
    AssetVaultError(AssetVaultError),
}

impl Display for AccountBuildError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "account build error: ")?;
        match self {
            AccountBuildError::AccountInitSeedNotSet => {
                write!(f, "account initial seed for ID generation is required but not set")
            },
            AccountBuildError::AccountSeedGenerationFailure(account_error) => {
                write!(f, "account seed generation failed: {account_error}")
            },
            AccountBuildError::ComponentInitializationError(account_error) => {
                write!(f, "account components failed to build: {account_error}")
            },
            AccountBuildError::AssetVaultError(asset_vault_error) => {
                write!(f, "account asset vault failed to build: {asset_vault_error}")
            },
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AccountBuildError {}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use assembly::{Assembler, Library};
    use vm_core::FieldElement;

    use super::*;
    use crate::accounts::StorageSlot;

    const CUSTOM_CODE1: &str = "
          export.foo
            push.2.2 add eq.4
          end
        ";
    const CUSTOM_CODE2: &str = "
            export.bar
              push.4.4 add eq.8
            end
          ";

    static CUSTOM_LIBRARY1: LazyLock<Library> = LazyLock::new(|| {
        Assembler::default()
            .assemble_library([CUSTOM_CODE1])
            .expect("code should be valid")
    });
    static CUSTOM_LIBRARY2: LazyLock<Library> = LazyLock::new(|| {
        Assembler::default()
            .assemble_library([CUSTOM_CODE2])
            .expect("code should be valid")
    });

    struct CustomComponent1 {
        slot0: u64,
    }
    impl From<CustomComponent1> for AccountComponent {
        fn from(custom: CustomComponent1) -> Self {
            let mut value = Word::default();
            value[0] = Felt::new(custom.slot0);

            AccountComponent::new(CUSTOM_LIBRARY1.clone(), vec![StorageSlot::Value(value)])
                .expect("component should be valid")
                .with_supports_all_types()
        }
    }

    struct CustomComponent2 {
        slot0: u64,
        slot1: u64,
    }
    impl From<CustomComponent2> for AccountComponent {
        fn from(custom: CustomComponent2) -> Self {
            let mut value0 = Word::default();
            value0[3] = Felt::new(custom.slot0);
            let mut value1 = Word::default();
            value1[3] = Felt::new(custom.slot1);

            AccountComponent::new(
                CUSTOM_LIBRARY2.clone(),
                vec![StorageSlot::Value(value0), StorageSlot::Value(value1)],
            )
            .expect("component should be valid")
            .with_supports_all_types()
        }
    }

    #[test]
    fn account_builder() {
        let storage_slot0 = 25;
        let storage_slot1 = 12;
        let storage_slot2 = 42;
        let nonce = Felt::ONE;
        let vault = AssetVault::mock();

        let (account, seed) = Account::builder()
            .init_seed([5; 32])
            .with_component(CustomComponent1 { slot0: storage_slot0 })
            .with_component(CustomComponent2 {
                slot0: storage_slot1,
                slot1: storage_slot2,
            })
            .with_assets(vault.assets())
            .nonce(nonce)
            .build()
            .unwrap();

        assert_eq!(account.nonce(), nonce);

        let computed_id =
            AccountId::new(seed, account.code.commitment(), account.storage.commitment()).unwrap();
        assert_eq!(account.id(), computed_id);

        // The merged code should have one procedure from each library.
        assert_eq!(account.code.procedure_roots().count(), 2);

        let foo_root = CUSTOM_LIBRARY1.mast_forest()
            [CUSTOM_LIBRARY1.get_export_node_id(CUSTOM_LIBRARY1.exports().next().unwrap())]
        .digest();
        let bar_root = CUSTOM_LIBRARY2.mast_forest()
            [CUSTOM_LIBRARY2.get_export_node_id(CUSTOM_LIBRARY2.exports().next().unwrap())]
        .digest();

        let foo_procedure_info = &account
            .code()
            .procedures()
            .iter()
            .find(|info| info.mast_root() == &foo_root)
            .unwrap();
        assert_eq!(foo_procedure_info.storage_offset(), 0);
        assert_eq!(foo_procedure_info.storage_size(), 1);

        let bar_procedure_info = &account
            .code()
            .procedures()
            .iter()
            .find(|info| info.mast_root() == &bar_root)
            .unwrap();
        assert_eq!(bar_procedure_info.storage_offset(), 1);
        assert_eq!(bar_procedure_info.storage_size(), 2);

        assert_eq!(
            account.storage().get_item(0).unwrap(),
            [Felt::new(storage_slot0), Felt::new(0), Felt::new(0), Felt::new(0)].into()
        );
        assert_eq!(
            account.storage().get_item(1).unwrap(),
            [Felt::new(0), Felt::new(0), Felt::new(0), Felt::new(storage_slot1)].into()
        );
        assert_eq!(
            account.storage().get_item(2).unwrap(),
            [Felt::new(0), Felt::new(0), Felt::new(0), Felt::new(storage_slot2)].into()
        );

        assert_eq!(account.vault, vault);
    }
}
