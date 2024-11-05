use alloc::{boxed::Box, vec::Vec};

use vm_processor::Digest;

use crate::{
    accounts::{
        Account, AccountCode, AccountComponent, AccountId, AccountStorage, AccountStorageMode,
        AccountType,
    },
    assets::{Asset, AssetVault},
    AccountError, Felt, Word, ZERO,
};

/// A convenient builder for an [`Account`] allowing for safe construction of an account by
/// combining multiple [`AccountComponent`]s.
///
/// This will build a valid new account with these properties:
/// - An empty [`AssetVault`].
/// - The nonce set to [`ZERO`].
/// - A seed which results in an [`AccountId`] valid for the configured account type and storage
///   mode.
///
/// By default, the builder is initialized with:
/// - The `account_type` set to [`AccountType::RegularAccountUpdatableCode`].
/// - The `storage_mode` set to [`AccountStorageMode::Private`].
///
/// The methods that are required to be called are:
///
/// - [`AccountBuilder::init_seed`],
/// - [`AccountBuilder::with_component`], which must be called at least once.
///
/// Under the `testing` feature, it is possible to:
/// - Change the `nonce` to build an existing account.
/// - Set assets which will be placed in the account's vault.
#[derive(Debug, Clone)]
pub struct AccountBuilder {
    nonce: Felt,
    #[cfg(feature = "testing")]
    assets: Vec<Asset>,
    components: Vec<AccountComponent>,
    account_type: AccountType,
    storage_mode: AccountStorageMode,
    init_seed: Option<[u8; 32]>,
}

impl AccountBuilder {
    /// Creates a new builder for a single account.
    pub fn new() -> Self {
        Self {
            nonce: ZERO,
            #[cfg(feature = "testing")]
            assets: vec![],
            components: vec![],
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

    /// Adds an [`AccountComponent`] to the builder. This method can be called multiple times and
    /// **must be called at least once** since an account must export at least one procedure.
    ///
    /// All components will be merged to form the final code and storage of the built account.
    pub fn with_component(mut self, account_component: impl Into<AccountComponent>) -> Self {
        self.components.push(account_component.into());
        self
    }

    /// Builds the common parts of testing and non-testing code.
    fn build_inner(
        &self,
    ) -> Result<([u8; 32], AssetVault, AccountCode, AccountStorage), AccountError> {
        let init_seed = self.init_seed.ok_or(AccountError::BuildError(
            "init_seed must be set on the account builder".into(),
            None,
        ))?;

        let vault = if cfg!(feature = "testing") {
            AssetVault::new(&self.assets).map_err(|err| {
                AccountError::BuildError(format!("asset vault failed to build: {err}"), None)
            })?
        } else {
            AssetVault::default()
        };

        #[cfg(feature = "testing")]
        if self.nonce == ZERO && !vault.is_empty() {
            return Err(AccountError::BuildError(
                "account asset vault must be empty on new accounts".into(),
                None,
            ));
        }

        let (code, storage) =
            Account::initialize_from_components(self.account_type, &self.components).map_err(
                |err| {
                    AccountError::BuildError(
                        "account components failed to build".into(),
                        Some(Box::new(err)),
                    )
                },
            )?;

        Ok((init_seed, vault, code, storage))
    }

    /// Grinds a new [`AccountId`] using the `init_seed` as a starting point.
    fn grind_account_id(
        &self,
        init_seed: [u8; 32],
        code_commitment: Digest,
        storage_commitment: Digest,
    ) -> Result<(AccountId, Word), AccountError> {
        let seed = AccountId::get_account_seed(
            init_seed,
            self.account_type,
            self.storage_mode,
            code_commitment,
            storage_commitment,
        )
        .map_err(|err| {
            AccountError::BuildError("account seed generation failed".into(), Some(Box::new(err)))
        })?;

        let account_id = AccountId::new(seed, code_commitment, storage_commitment)
            .expect("get_account_seed should provide a suitable seed");

        Ok((account_id, seed))
    }

    /// Builds an [`Account`] out of the configured builder.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The init seed is not set.
    /// - Any of the components does not support the set account type.
    /// - The number of procedures in all merged components is 0 or exceeds
    ///   [`AccountCode::MAX_NUM_PROCEDURES`](crate::accounts::AccountCode::MAX_NUM_PROCEDURES).
    /// - Two or more libraries export a procedure with the same MAST root.
    /// - The number of [`StorageSlot`](crate::accounts::StorageSlot)s of all components exceeds
    ///   255.
    /// - [`MastForest::merge`](vm_processor::MastForest::merge) fails on the given components.
    /// - If duplicate assets were added to the builder (only under the `testing` feature).
    /// - If the vault is not empty on new accounts (only under the `testing` feature).
    pub fn build(self) -> Result<(Account, Word), AccountError> {
        let (init_seed, vault, code, storage) = self.build_inner()?;

        let (account_id, seed) =
            self.grind_account_id(init_seed, code.commitment(), storage.commitment())?;

        debug_assert_eq!(account_id.account_type(), self.account_type);
        debug_assert_eq!(account_id.storage_mode(), self.storage_mode);

        let account = Account::from_parts(account_id, vault, storage, code, self.nonce);

        Ok((account, seed))
    }

    /// The build method optimized for testing scenarios. The only difference between this method
    /// and the [`Self::build`] method is that when building existing accounts, this function
    /// returns `None` for the seed, skips the grinding of an account id and constructs one
    /// instead. Hence it is always preferable to use this method in testing code.
    ///
    /// For possible errors, see the documentation of [`Self::build`].
    #[cfg(feature = "testing")]
    pub fn build_testing(self) -> Result<(Account, Option<Word>), AccountError> {
        let (init_seed, vault, code, storage) = self.build_inner()?;

        let (account_id, seed) = if self.nonce == ZERO {
            let (account_id, seed) =
                self.grind_account_id(init_seed, code.commitment(), storage.commitment())?;

            (account_id, Some(seed))
        } else {
            let account_id =
                Self::construct_account_id(self.account_type, self.storage_mode, init_seed);

            (account_id, None)
        };

        let account = Account::from_parts(account_id, vault, storage, code, self.nonce);

        Ok((account, seed))
    }
}

#[cfg(feature = "testing")]
impl AccountBuilder {
    /// Sets the nonce of the account. This method is optional.
    ///
    /// If unset, the nonce will default to [`ZERO`].
    pub fn nonce(mut self, nonce: Felt) -> Self {
        self.nonce = nonce;
        self
    }

    /// Adds all the assets to the account's [`AssetVault`]. This method is optional.
    ///
    /// Must only be called when nonce is non-[`ZERO`] since new accounts must have an empty vault.
    pub fn with_assets<I: IntoIterator<Item = Asset>>(mut self, assets: I) -> Self {
        self.assets.extend(assets);
        self
    }

    /// Constructs an [`AccountId`] for testing purposes with the given account type and storage
    /// mode and using the first 8 bytes of the `init_seed` as part of the account id.
    fn construct_account_id(
        account_type: AccountType,
        storage_mode: AccountStorageMode,
        init_seed: [u8; 32],
    ) -> AccountId {
        let id_high_nibble = (storage_mode as u8) << 6 | (account_type as u8) << 4;

        let mut bytes =
            <[u8; 8]>::try_from(&init_seed[0..8]).expect("we have sliced exactly 8 bytes off");

        // Clear the highest five bits of the most significant byte.
        // The high nibble must be cleared so we can set it to the storage mode and account type
        // we've constructed.
        // The 5th most significant bit is cleared to ensure the resulting id is a valid Felt even
        // when all other bits are set.
        bytes[0] &= 0x07;
        // Set high nibble of the most significant byte.
        bytes[0] |= id_high_nibble;

        let account_id = Felt::try_from(u64::from_be_bytes(bytes))
            .expect("must be a valid felt after clearing the 5th highest bit");
        let account_id = AccountId::new_unchecked(account_id);

        debug_assert_eq!(account_id.account_type(), account_type);
        debug_assert_eq!(account_id.storage_mode(), storage_mode);

        account_id
    }
}

impl Default for AccountBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// TESTS
// ================================================================================================

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

        let (account, seed) = Account::builder()
            .init_seed([5; 32])
            .with_component(CustomComponent1 { slot0: storage_slot0 })
            .with_component(CustomComponent2 {
                slot0: storage_slot1,
                slot1: storage_slot2,
            })
            .build()
            .unwrap();

        // Account should be new, i.e. nonce = zero.
        assert_eq!(account.nonce(), Felt::ZERO);

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
    }

    #[cfg(feature = "testing")]
    #[test]
    fn account_builder_non_empty_vault_on_new_account() {
        let storage_slot0 = 25;

        let build_error = Account::builder()
            .init_seed([0xff; 32])
            .with_component(CustomComponent1 { slot0: storage_slot0 })
            .with_assets(AssetVault::mock().assets())
            .build()
            .unwrap_err();

        assert!(
            matches!(build_error, AccountError::BuildError(msg, _) if msg == "account asset vault must be empty on new accounts")
        )
    }

    #[cfg(feature = "testing")]
    #[test]
    fn account_builder_id_construction() {
        // Use the highest possible input to check if the constructed id is a valid Felt in that
        // scenario.
        let init_seed = [0xff; 32];

        for account_type in [
            AccountType::FungibleFaucet,
            AccountType::NonFungibleFaucet,
            AccountType::RegularAccountImmutableCode,
            AccountType::RegularAccountUpdatableCode,
        ] {
            for storage_mode in [AccountStorageMode::Private, AccountStorageMode::Public] {
                // This function contains debug assertions already so we don't asset anything
                // additional
                AccountBuilder::construct_account_id(account_type, storage_mode, init_seed);
            }
        }
    }
}
