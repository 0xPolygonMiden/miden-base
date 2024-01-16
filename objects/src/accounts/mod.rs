use super::{
    assembly::{Assembler, AssemblyContext, ModuleAst},
    assets::AssetVault,
    utils::{
        collections::{BTreeMap, Vec},
        serde::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
        string::{String, ToString},
    },
    AccountError, Digest, Felt, FieldElement, Hasher, StarkField, Word, ZERO,
};

mod account_id;
pub use account_id::{compute_digest, digest_pow, validate_account_seed, AccountId, AccountType};

mod code;
pub use code::AccountCode;

pub mod delta;
pub use delta::{AccountDelta, AccountStorageDelta, AccountVaultDelta};

mod seed;
pub use seed::get_account_seed;

mod storage;
pub use storage::{AccountStorage, SlotItem, StorageSlotType};

mod stub;
pub use stub::AccountStub;

// TESTING CONSTANTS
// ================================================================================================

#[cfg(any(feature = "testing", test))]
pub const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN: u64 = 0b0110011011u64 << 54;

#[cfg(any(feature = "testing", test))]
pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN: u64 = 0b0001101110 << 54;

#[cfg(any(feature = "testing", test))]
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0b1010011100 << 54;

#[cfg(any(feature = "testing", test))]
pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN: u64 = 0b1101100110 << 54;

#[cfg(any(feature = "testing", test))]
pub const ACCOUNT_ID_INSUFFICIENT_ONES: u64 = 0b1100000110 << 54;

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
            if self.nonce.as_int() >= nonce.as_int() {
                return Err(AccountError::NonceNotMonotonicallyIncreasing {
                    current: self.nonce.as_int(),
                    new: nonce.as_int(),
                });
            }
            self.nonce = nonce;
        }

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
