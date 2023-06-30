use super::{
    delta::AccountDelta, hash_account, Account, AccountError, AccountId, AdviceProvider, Digest,
    Felt, TransactionResultError, TryFromVmResult, Vec, Word, WORD_SIZE,
};
use crypto::{
    merkle::{MerkleStore, StoreNode},
    utils::collections::{ApplyDiff, Diff},
};
use miden_core::StackOutputs;
use miden_lib::memory::{
    ACCT_CODE_ROOT_OFFSET, ACCT_DATA_MEM_SIZE, ACCT_ID_IDX, ACCT_NONCE_IDX,
    ACCT_STORAGE_ROOT_OFFSET, ACCT_VAULT_ROOT_OFFSET,
};

// ACCOUNT STUB
// ================================================================================================

/// A stub of an account which contains information that succinctly describes the state of the
/// components of the account.
///
/// The [AccountStub] is composed of:
/// - id: the account id ([AccountId]) of the account.
/// - nonce: the nonce of the account.
/// - vault_root: a commitment to the account's vault ([AccountVault]).
/// - vault_store: a [MerkleStore] that contains the account's vault Merkle data.
/// - storage_root: a commitment to the account's storage ([AccountStorage]).
/// - storage_store: a [MerkleStore] that contains the account's storage Merkle data.
/// - code_root: a commitment to the account's code ([AccountCode]).
/// - code_store: a [MerkleStore] that contains the account's code Merkle data.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountStub {
    id: AccountId,
    nonce: Felt,
    vault_root: Digest,
    vault_store: MerkleStore,
    storage_root: Digest,
    storage_store: MerkleStore,
    code_root: Digest,
    code_store: MerkleStore,
}

impl AccountStub {
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

    /// Returns a reference to the vault [MerkleStore] of this account.
    pub fn vault_store(&self) -> &MerkleStore {
        &self.vault_store
    }

    /// Returns the storage root of this account.
    pub fn storage_root(&self) -> Digest {
        self.storage_root
    }

    /// Returns a reference to the storage [MerkleStore] of this account.
    pub fn storage_store(&self) -> &MerkleStore {
        &self.storage_store
    }

    /// Returns the code root of this account.
    pub fn code_root(&self) -> Digest {
        self.code_root
    }

    /// Returns a reference to the code [MerkleStore] of this account.
    pub fn code_store(&self) -> &MerkleStore {
        &self.code_store
    }
}

// Diff IMPLEMENTATION
// ================================================================================================
impl Diff<Digest, StoreNode> for AccountStub {
    type DiffType = AccountDelta;

    fn diff(&self, other: &Self) -> Self::DiffType {
        let code_delta = match self.code_root() == other.code_root() {
            true => None,
            false => Some((other.code_root(), self.code_store.diff(&other.code_store))),
        };

        let nonce_delta = match self.nonce() == other.nonce() {
            true => None,
            false => Some(other.nonce()),
        };

        let storage_delta = match self.storage_root() == other.storage_root() {
            true => None,
            false => Some((other.storage_root(), self.storage_store.diff(&other.storage_store))),
        };

        let vault_delta = match self.vault_root() == other.vault_root() {
            true => None,
            false => Some((other.vault_root(), self.vault_store.diff(&other.vault_store))),
        };

        AccountDelta {
            code_delta,
            nonce_delta,
            storage_delta,
            vault_delta,
        }
    }
}

// ApplyDiff IMPLEMENTATION
// ================================================================================================
impl ApplyDiff<Digest, StoreNode> for AccountStub {
    type DiffType = AccountDelta;

    fn apply(&mut self, diff: Self::DiffType) {
        let AccountDelta {
            code_delta,
            nonce_delta,
            storage_delta,
            vault_delta,
        } = diff;

        // apply code diff if it exists
        if let Some((root, diff)) = code_delta {
            self.code_root = root;
            self.code_store.apply(diff);
        }

        // apply nonce diff
        if let Some(nonce) = nonce_delta {
            self.nonce = nonce;
        }

        // apply storage diff
        if let Some((root, diff)) = storage_delta {
            self.storage_root = root;
            self.storage_store.apply(diff);
        }

        // apply vault diff
        if let Some((root, diff)) = vault_delta {
            self.vault_root = root;
            self.vault_store.apply(diff);
        }
    }
}

// `TryFromVmResult` IMPLEMENTATION
// ================================================================================================

impl<T: AdviceProvider> TryFromVmResult<T> for AccountStub {
    type Error = TransactionResultError;

    fn try_from_vm_result(result: &StackOutputs, advice_provider: &T) -> Result<Self, Self::Error> {
        const FINAL_ACCOUNT_HASH_WORD_IDX: usize = 1;

        let final_account_hash: Word = result.stack()[FINAL_ACCOUNT_HASH_WORD_IDX * WORD_SIZE
            ..(FINAL_ACCOUNT_HASH_WORD_IDX + 1) * WORD_SIZE]
            .iter()
            .rev()
            .map(|x| Felt::from(*x))
            .collect::<Vec<_>>()
            .try_into()
            .expect("word size is correct");
        let final_account_hash: Digest = final_account_hash.into();

        // extract final account data from the advice map
        let final_account_data = advice_provider
            .get_mapped_values(&final_account_hash.as_bytes())
            .ok_or(TransactionResultError::FinalAccountDataNotFound)?;
        let (id, vault_root, storage_root, code_root, nonce) =
            parse_stub_account_commitments(final_account_data)
                .map_err(TransactionResultError::FinalAccountStubDataInvalid)?;

        // extract Merkle data
        let vault_store = advice_provider.get_store_subset([vault_root].iter());
        let storage_store = advice_provider.get_store_subset([storage_root].iter());
        let code_store = advice_provider.get_store_subset([code_root].iter());

        Ok(Self {
            id,
            nonce,
            vault_root,
            vault_store,
            storage_root,
            storage_store,
            code_root,
            code_store,
        })
    }
}

/// Parses the stub account data returned by the VM into individual account component commitments.
/// Returns a tuple of account ID, vault root, storage root, code root, and nonce.
fn parse_stub_account_commitments(
    elements: &[Felt],
) -> Result<(AccountId, Digest, Digest, Digest, Felt), AccountError> {
    if elements.len() != ACCT_DATA_MEM_SIZE * WORD_SIZE {
        return Err(AccountError::StubDataIncorrectLength(
            elements.len(),
            ACCT_DATA_MEM_SIZE * WORD_SIZE,
        ));
    }

    let id = AccountId::try_from(elements[ACCT_ID_IDX])?;
    let nonce = elements[ACCT_NONCE_IDX];
    let vault_root = TryInto::<Word>::try_into(
        &elements[ACCT_VAULT_ROOT_OFFSET as usize * WORD_SIZE
            ..(ACCT_VAULT_ROOT_OFFSET as usize + 1) * WORD_SIZE],
    )
    .expect("word is correct size")
    .into();
    let storage_root = TryInto::<Word>::try_into(
        &elements[ACCT_STORAGE_ROOT_OFFSET as usize * WORD_SIZE
            ..(ACCT_STORAGE_ROOT_OFFSET as usize + 1) * WORD_SIZE],
    )
    .expect("ord is correct size")
    .into();
    let code_root = TryInto::<Word>::try_into(
        &elements[ACCT_CODE_ROOT_OFFSET as usize * WORD_SIZE
            ..(ACCT_CODE_ROOT_OFFSET as usize + 1) * WORD_SIZE],
    )
    .expect("word is correct size")
    .into();

    Ok((id, vault_root, storage_root, code_root, nonce))
}

// `From<&Account> IMPLEMENTATION
// ================================================================================================

impl From<&Account> for AccountStub {
    fn from(account: &Account) -> Self {
        // extract Merkle data
        let vault_store = account.vault.asset_tree().into();
        let mut storage_store: MerkleStore = account.storage.slots().into();
        storage_store.extend(account.storage.store().inner_nodes());
        let code_store = account.code.procedure_tree().into();

        Self {
            id: account.id(),
            vault_root: account.vault().commitment(),
            vault_store,
            storage_root: account.storage().root(),
            storage_store,
            code_root: account.code().root(),
            code_store,
            nonce: account.nonce(),
        }
    }
}
