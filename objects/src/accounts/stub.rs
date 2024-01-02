use super::{hash_account, Account, AccountId, Digest, Felt};

// ACCOUNT STUB
// ================================================================================================

/// A stub of an account which contains information that succinctly describes the state of the
/// components of the account.
///
/// The [AccountStub] is composed of:
/// - id: the account id ([AccountId]) of the account.
/// - nonce: the nonce of the account.
/// - vault_root: a commitment to the account's vault ([AccountVault]).
/// - storage_root: accounts storage root ([AccountStorage]).
/// - code_root: a commitment to the account's code ([AccountCode]).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct AccountStub {
    id: AccountId,
    nonce: Felt,
    vault_root: Digest,
    storage_root: Digest,
    code_root: Digest,
}

impl AccountStub {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------
    /// Creates a new [AccountStub].
    pub fn new(
        id: AccountId,
        nonce: Felt,
        vault_root: Digest,
        storage_root: Digest,
        code_root: Digest,
    ) -> Self {
        Self {
            id,
            nonce,
            vault_root,
            storage_root,
            code_root,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------
    /// Returns hash of this account.
    ///
    /// Hash of an account is computed as hash(id, nonce, vault_root, storage_root, code_root).
    /// Computing the account hash requires 2 permutations of the hash function.
    pub fn hash(&self) -> Digest {
        hash_account(self.id, self.nonce, self.vault_root, self.storage_root, self.code_root)
    }

    /// Returns the id of this account.
    pub fn id(&self) -> AccountId {
        self.id
    }

    /// Returns the nonce of this account.
    pub fn nonce(&self) -> Felt {
        self.nonce
    }

    /// Returns the vault root of this account.
    pub fn vault_root(&self) -> Digest {
        self.vault_root
    }

    /// Returns the storage root of this account.
    pub fn storage_root(&self) -> Digest {
        self.storage_root
    }

    /// Returns the code root of this account.
    pub fn code_root(&self) -> Digest {
        self.code_root
    }
}

impl From<Account> for AccountStub {
    fn from(value: Account) -> Self {
        Self {
            id: value.id(),
            nonce: value.nonce(),
            vault_root: value.vault().commitment(),
            storage_root: value.storage().root(),
            code_root: value.code().root(),
        }
    }
}
