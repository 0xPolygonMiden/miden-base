use crate::{
    AccountError, Digest, Felt, Hasher, Word, ZERO,
    asset::AssetVault,
    utils::serde::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
};

mod account_id;
pub use account_id::{
    AccountId, AccountIdPrefix, AccountIdPrefixV0, AccountIdV0, AccountIdVersion,
    AccountStorageMode, AccountType, AddressType, NetworkId,
};

pub mod auth;

pub use auth::AuthSecretKey;

mod builder;
pub use builder::AccountBuilder;

pub mod code;
pub use code::{AccountCode, procedure::AccountProcedureInfo};

pub mod component;
pub use component::{
    AccountComponent, AccountComponentMetadata, AccountComponentTemplate, FeltRepresentation,
    InitStorageData, MapEntry, MapRepresentation, PlaceholderTypeRequirement, StorageEntry,
    StorageValueName, StorageValueNameError, TemplateType, TemplateTypeError, WordRepresentation,
};

pub mod delta;
pub use delta::{
    AccountDelta, AccountStorageDelta, AccountVaultDelta, FungibleAssetDelta,
    NonFungibleAssetDelta, NonFungibleDeltaAction, StorageMapDelta,
};

mod storage;
pub use storage::{
    AccountStorage, AccountStorageHeader, PartialStorage, StorageMap, StorageSlot, StorageSlotType,
};

mod header;
pub use header::AccountHeader;

mod file;
pub use file::AccountFile;

mod partial;
pub use partial::PartialAccount;

// ACCOUNT
// ================================================================================================

/// An account which can store assets and define rules for manipulating them.
///
/// An account consists of the following components:
/// - Account ID, which uniquely identifies the account and also defines basic properties of the
///   account.
/// - Account vault, which stores assets owned by the account.
/// - Account storage, which is a key-value map (both keys and values are words) used to store
///   arbitrary user-defined data.
/// - Account code, which is a set of Miden VM programs defining the public interface of the
///   account.
/// - Account nonce, a value which is incremented whenever account state is updated.
///
/// Out of the above components account ID is always immutable (once defined it can never be
/// changed). Other components may be mutated throughout the lifetime of the account. However,
/// account state can be changed only by invoking one of account interface methods.
///
/// The recommended way to build an account is through an [`AccountBuilder`], which can be
/// instantiated through [`Account::builder`]. See the type's documentation for details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    id: AccountId,
    vault: AssetVault,
    storage: AccountStorage,
    code: AccountCode,
    nonce: Felt,
}

impl Account {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns an [Account] instantiated with the provided components.
    pub fn from_parts(
        id: AccountId,
        vault: AssetVault,
        storage: AccountStorage,
        code: AccountCode,
        nonce: Felt,
    ) -> Self {
        Self { id, vault, storage, code, nonce }
    }

    /// Creates an account's [`AccountCode`] and [`AccountStorage`] from the provided components.
    ///
    /// This merges all libraries of the components into a single
    /// [`MastForest`](vm_processor::MastForest) to produce the [`AccountCode`]. For each
    /// procedure in the resulting forest, the storage offset and size are set so that the
    /// procedure can only access the storage slots of the component in which it was defined and
    /// each component's storage offset is the total number of slots in the previous components.
    /// To illustrate, given two components with one and two storage slots respectively:
    ///
    /// - RpoFalcon512 Component: Component slot 0 stores the public key.
    /// - Custom Component: Component slot 0 stores a custom [`StorageSlot::Value`] and component
    ///   slot 1 stores a custom [`StorageSlot::Map`].
    ///
    /// When combined, their assigned slots in the [`AccountStorage`] would be:
    ///
    /// - The RpoFalcon512 Component has offset 0 and size 1: Account slot 0 stores the public key.
    /// - The Custom Component has offset 1 and size 2: Account slot 1 stores the value and account
    ///   slot 2 stores the map.
    ///
    /// The resulting commitments from code and storage can then be used to construct an
    /// [`AccountId`]. Finally, a new account can then be instantiated from those parts using
    /// [`Account::new`].
    ///
    /// If the account type is faucet the reserved slot (slot 0) will be initialized.
    /// - For Fungible Faucets the value is [`StorageSlot::empty_value`].
    /// - For Non-Fungible Faucets the value is [`StorageSlot::empty_map`].
    ///
    /// If the storage needs to be initialized with certain values in that slot, those can be added
    /// after construction with the standard set methods for items and maps.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any of the components does not support `account_type`.
    /// - The number of procedures in all merged libraries is 0 or exceeds
    ///   [`AccountCode::MAX_NUM_PROCEDURES`].
    /// - Two or more libraries export a procedure with the same MAST root.
    /// - The number of [`StorageSlot`]s of all components exceeds 255.
    /// - [`MastForest::merge`](vm_processor::MastForest::merge) fails on all libraries.
    pub(super) fn initialize_from_components(
        account_type: AccountType,
        components: &[AccountComponent],
    ) -> Result<(AccountCode, AccountStorage), AccountError> {
        validate_components_support_account_type(components, account_type)?;

        let code = AccountCode::from_components_unchecked(components, account_type)?;
        let storage = AccountStorage::from_components(components, account_type)?;

        Ok((code, storage))
    }

    /// Creates a new [`AccountBuilder`] for an account and sets the initial seed from which the
    /// grinding process for that account's [`AccountId`] will start.
    ///
    /// This initial seed should come from a cryptographic random number generator.
    pub fn builder(init_seed: [u8; 32]) -> AccountBuilder {
        AccountBuilder::new(init_seed)
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the commitment of this account.
    ///
    /// The commitment of an account is computed as hash(id, nonce, vault_root, storage_commitment,
    /// code_commitment). Computing the account commitment requires 2 permutations of the hash
    /// function.
    pub fn commitment(&self) -> Digest {
        hash_account(
            self.id,
            self.nonce,
            self.vault.root(),
            self.storage.commitment(),
            self.code.commitment(),
        )
    }

    /// Returns the commitment of this account as used for the initial account state commitment in
    /// transaction proofs.
    ///
    /// For existing accounts, this is exactly the same as [Account::commitment()], however, for new
    /// accounts this value is set to [crate::EMPTY_WORD]. This is because when a transaction is
    /// executed against a new account, public input for the initial account state is set to
    /// [crate::EMPTY_WORD] to distinguish new accounts from existing accounts. The actual
    /// commitment of the initial account state (and the initial state itself), are provided to
    /// the VM via the advice provider.
    pub fn init_commitment(&self) -> Digest {
        if self.is_new() {
            Digest::default()
        } else {
            self.commitment()
        }
    }

    /// Returns unique identifier of this account.
    pub fn id(&self) -> AccountId {
        self.id
    }

    /// Returns the account type
    pub fn account_type(&self) -> AccountType {
        self.id.account_type()
    }

    /// Returns a reference to the vault of this account.
    pub fn vault(&self) -> &AssetVault {
        &self.vault
    }

    /// Returns a reference to the storage of this account.
    pub fn storage(&self) -> &AccountStorage {
        &self.storage
    }

    /// Returns a reference to the code of this account.
    pub fn code(&self) -> &AccountCode {
        &self.code
    }

    /// Returns nonce for this account.
    pub fn nonce(&self) -> Felt {
        self.nonce
    }

    /// Returns true if this account can issue assets.
    pub fn is_faucet(&self) -> bool {
        self.id.is_faucet()
    }

    /// Returns true if this is a regular account.
    pub fn is_regular_account(&self) -> bool {
        self.id.is_regular_account()
    }

    /// Returns `true` if the full state of the account is on chain, i.e. if the storage modes are
    /// [`AccountStorageMode::Public`] or [`AccountStorageMode::Network`], `false` otherwise.
    pub fn is_onchain(&self) -> bool {
        self.id().is_onchain()
    }

    /// Returns `true` if the storage mode is [`AccountStorageMode::Public`], `false` otherwise.
    pub fn is_public(&self) -> bool {
        self.id().is_public()
    }

    /// Returns `true` if the storage mode is [`AccountStorageMode::Private`], `false` otherwise.
    pub fn is_private(&self) -> bool {
        self.id().is_private()
    }

    /// Returns `true` if the storage mode is [`AccountStorageMode::Network`], `false` otherwise.
    pub fn is_network(&self) -> bool {
        self.id().is_network()
    }

    /// Returns true if the account is new (i.e. it has not been initialized yet).
    pub fn is_new(&self) -> bool {
        self.nonce == ZERO
    }

    /// Decomposes the account into the underlying account components.
    pub fn into_parts(self) -> (AccountId, AssetVault, AccountStorage, AccountCode, Felt) {
        (self.id, self.vault, self.storage, self.code, self.nonce)
    }

    // DATA MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Applies the provided delta to this account. This updates account vault, storage, and nonce
    /// to the values specified by the delta.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Applying vault sub-delta to the vault of this account fails.
    /// - Applying storage sub-delta to the storage of this account fails.
    /// - The nonce specified in the provided delta smaller than or equal to the current account
    ///   nonce.
    pub fn apply_delta(&mut self, delta: &AccountDelta) -> Result<(), AccountError> {
        // update vault; we don't check vault delta validity here because `AccountDelta` can contain
        // only valid vault deltas
        self.vault
            .apply_delta(delta.vault())
            .map_err(AccountError::AssetVaultUpdateError)?;

        // update storage
        self.storage.apply_delta(delta.storage())?;

        // update nonce
        if let Some(nonce) = delta.nonce() {
            self.set_nonce(nonce)?;
        }

        Ok(())
    }

    /// Sets the nonce of this account to the specified nonce value.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The new nonce is smaller than the actual account nonce
    /// - The new nonce is equal to the actual account nonce
    pub fn set_nonce(&mut self, nonce: Felt) -> Result<(), AccountError> {
        if self.nonce.as_int() >= nonce.as_int() {
            return Err(AccountError::NonceNotMonotonicallyIncreasing {
                current: self.nonce.as_int(),
                new: nonce.as_int(),
            });
        }

        self.nonce = nonce;

        Ok(())
    }

    // TEST HELPERS
    // --------------------------------------------------------------------------------------------

    #[cfg(any(feature = "testing", test))]
    /// Returns a mutable reference to the vault of this account.
    pub fn vault_mut(&mut self) -> &mut AssetVault {
        &mut self.vault
    }

    #[cfg(any(feature = "testing", test))]
    /// Returns a mutable reference to the storage of this account.
    pub fn storage_mut(&mut self) -> &mut AccountStorage {
        &mut self.storage
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for Account {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let Account { id, vault, storage, code, nonce } = self;

        id.write_into(target);
        vault.write_into(target);
        storage.write_into(target);
        code.write_into(target);
        nonce.write_into(target);
    }

    fn get_size_hint(&self) -> usize {
        self.id.get_size_hint()
            + self.vault.get_size_hint()
            + self.storage.get_size_hint()
            + self.code.get_size_hint()
            + self.nonce.get_size_hint()
    }
}

impl Deserializable for Account {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let id = AccountId::read_from(source)?;
        let vault = AssetVault::read_from(source)?;
        let storage = AccountStorage::read_from(source)?;
        let code = AccountCode::read_from(source)?;
        let nonce = Felt::read_from(source)?;

        Ok(Self::from_parts(id, vault, storage, code, nonce))
    }
}

// HELPERS
// ================================================================================================

/// Returns hash of an account with the specified ID, nonce, vault root, storage commitment, and
/// code commitment.
///
/// Hash of an account is computed as hash(id, nonce, vault_root, storage_commitment,
/// code_commitment). Computing the account commitment requires 2 permutations of the hash function.
pub fn hash_account(
    id: AccountId,
    nonce: Felt,
    vault_root: Digest,
    storage_commitment: Digest,
    code_commitment: Digest,
) -> Digest {
    let mut elements = [ZERO; 16];
    elements[0] = id.suffix();
    elements[1] = id.prefix().as_felt();
    elements[3] = nonce;
    elements[4..8].copy_from_slice(&*vault_root);
    elements[8..12].copy_from_slice(&*storage_commitment);
    elements[12..].copy_from_slice(&*code_commitment);
    Hasher::hash_elements(&elements)
}

/// Validates that all `components` support the given `account_type`.
fn validate_components_support_account_type(
    components: &[AccountComponent],
    account_type: AccountType,
) -> Result<(), AccountError> {
    for (component_index, component) in components.iter().enumerate() {
        if !component.supports_type(account_type) {
            return Err(AccountError::UnsupportedComponentForAccountType {
                account_type,
                component_index,
            });
        }
    }

    Ok(())
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use assembly::Assembler;
    use assert_matches::assert_matches;
    use miden_crypto::{
        Felt, Word,
        utils::{Deserializable, Serializable},
    };
    use vm_processor::Digest;

    use super::{
        AccountCode, AccountDelta, AccountId, AccountStorage, AccountStorageDelta,
        AccountVaultDelta,
    };
    use crate::{
        AccountError,
        account::{
            Account, AccountComponent, AccountType, StorageMap, StorageMapDelta, StorageSlot,
        },
        asset::{Asset, AssetVault, FungibleAsset, NonFungibleAsset},
        testing::{
            account_id::ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE,
            storage::AccountStorageDeltaBuilder,
        },
    };

    #[test]
    fn test_serde_account() {
        let init_nonce = Felt::new(1);
        let asset_0 = FungibleAsset::mock(99);
        let word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
        let storage_slot = StorageSlot::Value(word);
        let account = build_account(vec![asset_0], init_nonce, vec![storage_slot]);

        let serialized = account.to_bytes();
        let deserialized = Account::read_from_bytes(&serialized).unwrap();
        assert_eq!(deserialized, account);
    }

    #[test]
    fn test_serde_account_delta() {
        let final_nonce = Felt::new(2);
        let asset_0 = FungibleAsset::mock(15);
        let asset_1 = NonFungibleAsset::mock(&[5, 5, 5]);
        let storage_delta = AccountStorageDeltaBuilder::default()
            .add_cleared_items([0])
            .add_updated_values([(1_u8, [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)])])
            .build()
            .unwrap();
        let account_delta =
            build_account_delta(vec![asset_1], vec![asset_0], final_nonce, storage_delta);

        let serialized = account_delta.to_bytes();
        let deserialized = AccountDelta::read_from_bytes(&serialized).unwrap();
        assert_eq!(deserialized, account_delta);
    }

    #[test]
    fn valid_account_delta_is_correctly_applied() {
        // build account
        let init_nonce = Felt::new(1);
        let asset_0 = FungibleAsset::mock(100);
        let asset_1 = NonFungibleAsset::mock(&[1, 2, 3]);

        // build storage slots
        let storage_slot_value_0 =
            StorageSlot::Value([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]);
        let storage_slot_value_1 =
            StorageSlot::Value([Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)]);
        let mut storage_map = StorageMap::with_entries([
            (
                Digest::new([Felt::new(101), Felt::new(102), Felt::new(103), Felt::new(104)]),
                [Felt::new(1_u64), Felt::new(2_u64), Felt::new(3_u64), Felt::new(4_u64)],
            ),
            (
                Digest::new([Felt::new(105), Felt::new(106), Felt::new(107), Felt::new(108)]),
                [Felt::new(5_u64), Felt::new(6_u64), Felt::new(7_u64), Felt::new(8_u64)],
            ),
        ])
        .unwrap();
        let storage_slot_map = StorageSlot::Map(storage_map.clone());

        let mut account = build_account(
            vec![asset_0],
            init_nonce,
            vec![storage_slot_value_0, storage_slot_value_1, storage_slot_map],
        );

        // update storage map
        let new_map_entry = (
            Digest::new([Felt::new(101), Felt::new(102), Felt::new(103), Felt::new(104)]),
            [Felt::new(9_u64), Felt::new(10_u64), Felt::new(11_u64), Felt::new(12_u64)],
        );

        let updated_map =
            StorageMapDelta::from_iters([], [(new_map_entry.0.into(), new_map_entry.1)]);
        storage_map.insert(new_map_entry.0, new_map_entry.1);

        // build account delta
        let final_nonce = Felt::new(2);
        let storage_delta = AccountStorageDeltaBuilder::default()
            .add_cleared_items([0])
            .add_updated_values([(1, [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)])])
            .add_updated_maps([(2, updated_map)])
            .build()
            .unwrap();
        let account_delta =
            build_account_delta(vec![asset_1], vec![asset_0], final_nonce, storage_delta);

        // apply delta and create final_account
        account.apply_delta(&account_delta).unwrap();

        let final_account = build_account(
            vec![asset_1],
            final_nonce,
            vec![
                StorageSlot::Value(Word::default()),
                StorageSlot::Value([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
                StorageSlot::Map(storage_map),
            ],
        );

        // assert account is what it should be
        assert_eq!(account, final_account);
    }

    #[test]
    #[should_panic]
    fn valid_account_delta_with_unchanged_nonce() {
        // build account
        let init_nonce = Felt::new(1);
        let asset = FungibleAsset::mock(110);
        let mut account =
            build_account(vec![asset], init_nonce, vec![StorageSlot::Value(Word::default())]);

        // build account delta
        let storage_delta = AccountStorageDeltaBuilder::default()
            .add_cleared_items([0])
            .add_updated_values([(1_u8, [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)])])
            .build()
            .unwrap();
        let account_delta = build_account_delta(vec![], vec![asset], init_nonce, storage_delta);

        // apply delta
        account.apply_delta(&account_delta).unwrap()
    }

    #[test]
    #[should_panic]
    fn valid_account_delta_with_decremented_nonce() {
        // build account
        let init_nonce = Felt::new(2);
        let asset = FungibleAsset::mock(100);
        let mut account =
            build_account(vec![asset], init_nonce, vec![StorageSlot::Value(Word::default())]);

        // build account delta
        let final_nonce = Felt::new(1);
        let storage_delta = AccountStorageDeltaBuilder::default()
            .add_cleared_items([0])
            .add_updated_values([(1_u8, [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)])])
            .build()
            .unwrap();
        let account_delta = build_account_delta(vec![], vec![asset], final_nonce, storage_delta);

        // apply delta
        account.apply_delta(&account_delta).unwrap()
    }

    #[test]
    fn empty_account_delta_with_incremented_nonce() {
        // build account
        let init_nonce = Felt::new(1);
        let word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
        let storage_slot = StorageSlot::Value(word);
        let mut account = build_account(vec![], init_nonce, vec![storage_slot]);

        // build account delta
        let final_nonce = Felt::new(2);
        let account_delta = AccountDelta::new(
            AccountStorageDelta::default(),
            AccountVaultDelta::default(),
            Some(final_nonce),
        )
        .unwrap();

        // apply delta
        account.apply_delta(&account_delta).unwrap()
    }

    pub fn build_account_delta(
        added_assets: Vec<Asset>,
        removed_assets: Vec<Asset>,
        nonce: Felt,
        storage_delta: AccountStorageDelta,
    ) -> AccountDelta {
        let vault_delta = AccountVaultDelta::from_iters(added_assets, removed_assets);
        AccountDelta::new(storage_delta, vault_delta, Some(nonce)).unwrap()
    }

    pub fn build_account(assets: Vec<Asset>, nonce: Felt, slots: Vec<StorageSlot>) -> Account {
        let id = AccountId::try_from(ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE).unwrap();
        let code = AccountCode::mock();

        let vault = AssetVault::new(&assets).unwrap();

        let storage = AccountStorage::new(slots).unwrap();

        Account::from_parts(id, vault, storage, code, nonce)
    }

    /// Tests that initializing code and storage from a component which does not support the given
    /// account type returns an error.
    #[test]
    fn test_account_unsupported_component_type() {
        let code1 = "export.foo add end";
        let library1 = Assembler::default().assemble_library([code1]).unwrap();

        // This component support all account types except the regular account with updatable code.
        let component1 = AccountComponent::new(library1, vec![])
            .unwrap()
            .with_supported_type(AccountType::FungibleFaucet)
            .with_supported_type(AccountType::NonFungibleFaucet)
            .with_supported_type(AccountType::RegularAccountImmutableCode);

        let err = Account::initialize_from_components(
            AccountType::RegularAccountUpdatableCode,
            &[component1],
        )
        .unwrap_err();

        assert!(matches!(
            err,
            AccountError::UnsupportedComponentForAccountType {
                account_type: AccountType::RegularAccountUpdatableCode,
                component_index: 0
            }
        ))
    }

    /// Two components who export a procedure with the same MAST root should fail to convert into
    /// code and storage.
    #[test]
    fn test_account_duplicate_exported_mast_root() {
        let code1 = "export.foo add eq.1 end";
        let code2 = "export.bar add eq.1 end";

        let library1 = Assembler::default().assemble_library([code1]).unwrap();
        let library2 = Assembler::default().assemble_library([code2]).unwrap();

        let component1 = AccountComponent::new(library1, vec![]).unwrap().with_supports_all_types();
        let component2 = AccountComponent::new(library2, vec![]).unwrap().with_supports_all_types();

        let err = Account::initialize_from_components(
            AccountType::RegularAccountUpdatableCode,
            &[component1, component2],
        )
        .unwrap_err();

        assert_matches!(err, AccountError::AccountComponentDuplicateProcedureRoot(_))
    }
}
