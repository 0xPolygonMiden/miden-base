use super::{assets::Asset, AccountError, Digest, Felt, Hasher, StarkField, Word, ZERO};

mod account_id;
pub use account_id::AccountId;

mod code;
pub use code::AccountCode;

mod storage;
pub use storage::AccountStorage;
use storage::StorageItem;

mod vault;
pub use vault::AccountVault;

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
    vault: AccountVault,
    storage: AccountStorage,
    code: AccountCode,
    nonce: Felt,
}

impl Account {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates and returns a new account initialized with the specified ID, storage items, and
    /// code.
    ///
    /// The vault of the account is initially empty and nonce is set to ZERO.
    ///
    /// # Errors
    /// Returns an error if compilation of the source code fails.
    pub fn new(
        id: AccountId,
        storage_items: &[StorageItem],
        code_source: &str,
    ) -> Result<Self, AccountError> {
        Ok(Self {
            id,
            vault: AccountVault::default(),
            storage: AccountStorage::new(storage_items),
            code: AccountCode::new(code_source)?,
            nonce: ZERO,
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns hash of this account.
    ///
    /// Hash of an account is computed as hash(id, nonce, vault_root, storage_root, code_root).
    /// Computing the account hash requires 2 permutations of the hash function.
    pub fn hash(&self) -> Digest {
        let mut elements = [ZERO; 16];
        elements[..3].copy_from_slice(self.id.as_elements());
        elements[3] = self.nonce;
        elements[4..8].copy_from_slice(self.vault.root().as_elements());
        elements[8..12].copy_from_slice(self.storage.root().as_elements());
        elements[12..].copy_from_slice(self.code.root().as_elements());
        Hasher::hash_elements(&elements)
    }

    /// Returns unique identifier of this account.
    pub fn id(&self) -> AccountId {
        self.id
    }

    /// Returns a reference to the vault of this account.
    pub fn vault(&self) -> &AccountVault {
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

    /// Returns true if this account can issue fungible assets.
    pub fn is_fungible_faucet(&self) -> bool {
        self.id.is_fungible_faucet()
    }

    /// Returns true if this account can issue non-fungible assets.
    pub fn is_non_fungible_faucet(&self) -> bool {
        self.id.is_non_fungible_faucet()
    }
}
