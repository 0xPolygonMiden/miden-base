use crate::{
    assembly::{Assembler, AssemblyContext, ModuleAst},
    assets::AssetVault,
    utils::serde::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    AccountError, Digest, Felt, Hasher, Word, ZERO,
};

mod account_id;
pub use account_id::{
    AccountId, AccountStorageType, AccountType, ACCOUNT_ISFAUCET_MASK, ACCOUNT_STORAGE_MASK_SHIFT,
    ACCOUNT_TYPE_MASK_SHIFT,
};

mod code;
pub use code::AccountCode;

pub mod delta;
pub use delta::{AccountDelta, AccountStorageDelta, AccountVaultDelta};

mod seed;
pub use seed::{get_account_seed, get_account_seed_single};

mod storage;
pub use storage::{AccountStorage, SlotItem, StorageSlot, StorageSlotType};

mod stub;
pub use stub::AccountStub;

mod data;
pub use data::{AccountData, AuthData};

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
/// Out of the the above components account ID is always immutable (once defined it can never be
/// changed). Other components may be mutated throughout the lifetime of the account. However,
/// account state can be changed only by invoking one of account interface methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    id: AccountId,
    vault: AssetVault,
    storage: AccountStorage,
    code: AccountCode,
    nonce: Felt,
}

impl Account {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates and returns a new account initialized with the specified ID, vault, storage, code,
    /// and nonce.
    pub fn new(
        id: AccountId,
        vault: AssetVault,
        storage: AccountStorage,
        code: AccountCode,
        nonce: Felt,
    ) -> Self {
        Self { id, vault, storage, code, nonce }
    }

    /// Creates and returns a new account constructed from the specified ID and delta.
    pub fn from_delta(id: AccountId, delta: &AccountDelta) -> Result<Self, AccountError> {
        let vault = AssetVault::new(&delta.vault().added_assets)
            .map_err(|error| AccountError::AssetVaultCreateError { account_id: id, error })?;
        let storage = AccountStorage::new(
            delta
                .storage()
                .updated_items
                .iter()
                .map(|&(index, value)| SlotItem {
                    index,
                    slot: StorageSlot::new_value(value),
                })
                .collect(),
        )?;
        let code = delta.code().ok_or(AccountError::NewOnChainAccountMissingCode(id))?.clone();
        let nonce = delta.nonce().ok_or(AccountError::NewOnChainAccountMissingNonce(id))?;

        Ok(Self { id, vault, storage, code, nonce })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns hash of this account.
    ///
    /// Hash of an account is computed as hash(id, nonce, vault_root, storage_root, code_root).
    /// Computing the account hash requires 2 permutations of the hash function.
    pub fn hash(&self) -> Digest {
        hash_account(
            self.id,
            self.nonce,
            self.vault.commitment(),
            self.storage.root(),
            self.code.root(),
        )
    }

    /// Returns hash of this account as used for the initial account state hash in transaction
    /// proofs.
    ///
    /// For existing accounts, this is exactly the same as [Account::hash()], however, for new
    /// accounts this value is set to [ZERO; 4]. This is because when a transaction is executed
    /// against a new account, public input for the initial account state is set to [ZERO; 4] to
    /// distinguish new accounts from existing accounts. The actual hash of the initial account
    /// state (and the initial state itself), are provided to the VM via the advice provider.
    pub fn proof_init_hash(&self) -> Digest {
        if self.is_new() {
            Digest::default()
        } else {
            self.hash()
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

    /// Returns true if this account is on-chain.
    pub fn is_on_chain(&self) -> bool {
        self.id.is_on_chain()
    }

    /// Returns true if the account is new (i.e. it has not been initialized yet).
    pub fn is_new(&self) -> bool {
        self.nonce == ZERO
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
        // update vault; we don't check vault delta validity here because AccountDelta can contain
        // only valid vault deltas
        for &asset in delta.vault().added_assets.iter() {
            self.vault.add_asset(asset).map_err(AccountError::AssetVaultUpdateError)?;
        }

        for &asset in delta.vault().removed_assets.iter() {
            self.vault.remove_asset(asset).map_err(AccountError::AssetVaultUpdateError)?;
        }

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

    #[cfg(test)]
    /// Returns a mutable reference to the vault of this account.
    pub fn vault_mut(&mut self) -> &mut AssetVault {
        &mut self.vault
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
}

impl Deserializable for Account {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let id = AccountId::read_from(source)?;
        let vault = AssetVault::read_from(source)?;
        let storage = AccountStorage::read_from(source)?;
        let code = AccountCode::read_from(source)?;
        let nonce = Felt::read_from(source)?;

        Ok(Self::new(id, vault, storage, code, nonce))
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Account {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let bytes = self.to_bytes();
        serializer.serialize_bytes(&bytes)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Account {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use alloc::vec::Vec;

        let bytes: Vec<u8> = <Vec<u8> as serde::Deserialize>::deserialize(deserializer)?;
        Self::read_from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

// HELPERS
// ================================================================================================

/// Returns hash of an account with the specified ID, nonce, vault root, storage root, and code root.
///
/// Hash of an account is computed as hash(id, nonce, vault_root, storage_root, code_root).
/// Computing the account hash requires 2 permutations of the hash function.
pub fn hash_account(
    id: AccountId,
    nonce: Felt,
    vault_root: Digest,
    storage_root: Digest,
    code_root: Digest,
) -> Digest {
    let mut elements = [ZERO; 16];
    elements[0] = id.into();
    elements[3] = nonce;
    elements[4..8].copy_from_slice(&*vault_root);
    elements[8..12].copy_from_slice(&*storage_root);
    elements[12..].copy_from_slice(&*code_root);
    Hasher::hash_elements(&elements)
}

// TESTING ACCOUNT IDs
// ================================================================================================

#[cfg(any(feature = "testing", test))]
pub use testing::*;

#[cfg(any(feature = "testing", test))]
mod testing {
    use super::{
        AccountStorageType, AccountType, ACCOUNT_STORAGE_MASK_SHIFT, ACCOUNT_TYPE_MASK_SHIFT,
    };

    const fn account_id(account_type: AccountType, storage: AccountStorageType, rest: u64) -> u64 {
        let mut id = 0;

        id ^= (storage as u64) << ACCOUNT_STORAGE_MASK_SHIFT;
        id ^= (account_type as u64) << ACCOUNT_TYPE_MASK_SHIFT;
        id ^= rest;

        id
    }

    // REGULAR ACCOUNTS - OFF-CHAIN
    pub const ACCOUNT_ID_SENDER: u64 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageType::OffChain,
        0b0001_1111,
    );
    pub const ACCOUNT_ID_OFF_CHAIN_SENDER: u64 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageType::OffChain,
        0b0010_1111,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN: u64 = account_id(
        AccountType::RegularAccountUpdatableCode,
        AccountStorageType::OffChain,
        0b0011_1111,
    );
    // REGULAR ACCOUNTS - ON-CHAIN
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN: u64 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageType::OnChain,
        0b0001_1111,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2: u64 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageType::OnChain,
        0b0010_1111,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: u64 = account_id(
        AccountType::RegularAccountUpdatableCode,
        AccountStorageType::OnChain,
        0b0011_1111,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2: u64 = account_id(
        AccountType::RegularAccountUpdatableCode,
        AccountStorageType::OnChain,
        0b0100_1111,
    );

    // FUNGIBLE TOKENS - OFF-CHAIN
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageType::OffChain, 0b0001_1111);
    // FUNGIBLE TOKENS - ON-CHAIN
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageType::OnChain, 0b0001_1111);
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageType::OnChain, 0b0010_1111);
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageType::OnChain, 0b0011_1111);
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageType::OnChain, 0b0100_1111);

    // NON-FUNGIBLE TOKENS - OFF-CHAIN
    pub const ACCOUNT_ID_INSUFFICIENT_ONES: u64 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageType::OffChain, 0b0000_0000); // invalid
    pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN: u64 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageType::OffChain, 0b0001_1111);
    // NON-FUNGIBLE TOKENS - ON-CHAIN
    pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN: u64 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageType::OnChain, 0b0010_1111);
    pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1: u64 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageType::OnChain, 0b0011_1111);

    #[test]
    fn test_account_id() {
        use crate::accounts::AccountId;

        for account_type in [
            AccountType::RegularAccountImmutableCode,
            AccountType::RegularAccountUpdatableCode,
            AccountType::NonFungibleFaucet,
            AccountType::FungibleFaucet,
        ] {
            for storage_type in [AccountStorageType::OnChain, AccountStorageType::OffChain] {
                let acc = AccountId::try_from(account_id(account_type, storage_type, 0b1111_1111))
                    .unwrap();
                assert_eq!(acc.account_type(), account_type);
                assert_eq!(acc.storage_type(), storage_type);
            }
        }
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::{
        Account, AccountCode, AccountDelta, AccountId, AccountStorage, AccountStorageDelta,
        AccountVaultDelta, Assembler, Felt, ModuleAst, SlotItem, StorageSlot, StorageSlotType,
        Word, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
    };
    use crate::assets::{Asset, AssetVault, FungibleAsset};

    fn build_account(assets: Vec<Asset>, nonce: Felt, storage_items: Vec<Word>) -> Account {
        // build account code
        let source = "
            export.foo
                push.1 push.2 mul
            end
            export.bar
                push.1 push.2 add
            end
        ";
        let module = ModuleAst::parse(source).unwrap();
        let assembler = Assembler::default();
        let code = AccountCode::new(module, &assembler).unwrap();

        // build account data
        let vault = AssetVault::new(&assets).unwrap();

        let slot_type = StorageSlotType::Value { value_arity: 0 };
        let slot_items: Vec<SlotItem> = storage_items
            .into_iter()
            .enumerate()
            .map(|(index, item)| SlotItem {
                index: index as u8,
                slot: StorageSlot { slot_type, value: item },
            })
            .collect();
        let storage = AccountStorage::new(slot_items).unwrap();

        // create account
        let id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap();
        Account::new(id, vault, storage, code, nonce)
    }

    fn build_account_delta(
        added_assets: Vec<Asset>,
        removed_assets: Vec<Asset>,
        nonce: Felt,
    ) -> AccountDelta {
        let word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
        let storage_delta = AccountStorageDelta {
            cleared_items: vec![0],
            updated_items: vec![(1, word)],
        };

        let vault_delta = AccountVaultDelta { added_assets, removed_assets };

        AccountDelta::new(storage_delta, vault_delta, None, Some(nonce)).unwrap()
    }

    fn build_assets() -> (Asset, Asset) {
        // build asset 0
        let faucet_id_0 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
        let asset_0: Asset = FungibleAsset::new(faucet_id_0, 123).unwrap().into();

        // build asset 1
        let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap();
        let asset_1: Asset = FungibleAsset::new(faucet_id_1, 345).unwrap().into();

        (asset_0, asset_1)
    }

    #[test]
    fn valid_account_delta_is_correctly_applied() {
        // build account
        let init_nonce = Felt::new(1);
        let (asset_0, asset_1) = build_assets();
        let word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
        let mut account = build_account(vec![asset_0], init_nonce, vec![word]);

        // build account delta
        let final_nonce = Felt::new(2);
        let account_delta = build_account_delta(vec![asset_1], vec![asset_0], final_nonce);

        // apply delta and create final_account
        account.apply_delta(&account_delta).unwrap();
        let final_account = build_account(vec![asset_1], final_nonce, vec![Word::default(), word]);

        // assert account is what it should be
        assert_eq!(account, final_account);
    }

    #[test]
    #[should_panic]
    fn valid_account_delta_with_unchanged_nonce() {
        // build account
        let init_nonce = Felt::new(1);
        let (asset, _) = build_assets();
        let mut account = build_account(vec![asset], init_nonce, vec![Word::default()]);

        // build account delta
        let account_delta = build_account_delta(vec![], vec![asset], init_nonce);

        // apply delta
        account.apply_delta(&account_delta).unwrap()
    }

    #[test]
    #[should_panic]
    fn valid_account_delta_with_decremented_nonce() {
        // build account
        let init_nonce = Felt::new(2);
        let (asset, _) = build_assets();
        let mut account = build_account(vec![asset], init_nonce, vec![Word::default()]);

        // build account delta
        let final_nonce = Felt::new(1);
        let account_delta = build_account_delta(vec![], vec![asset], final_nonce);

        // apply delta
        account.apply_delta(&account_delta).unwrap()
    }

    #[test]
    #[should_panic]
    fn empty_account_delta_with_incremented_nonce() {
        // build account
        let init_nonce = Felt::new(1);
        let word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
        let mut account = build_account(vec![], init_nonce, vec![word]);

        // build account delta
        let final_nonce = Felt::new(2);
        let account_delta = AccountDelta::new(
            AccountStorageDelta::default(),
            AccountVaultDelta::default(),
            None,
            Some(final_nonce),
        )
        .unwrap();

        // apply delta
        account.apply_delta(&account_delta).unwrap()
    }
}
