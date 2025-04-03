use miden_crypto::merkle::{MerkleError, Smt};

use crate::{
    Digest, Felt, FieldElement, Word,
    account::{AccountId, AccountIdPrefix},
    block::AccountWitness,
    errors::AccountTreeError,
};

/// The sparse merkle tree of all accounts in the blockchain.
///
/// The key is the [`AccountId`] while the value is the current state commitment of the account,
/// i.e. [`Account::commitment`](crate::account::Account::commitment)). If the account is new, then
/// the commitment is the [`EMPTY_WORD`](crate::EMPTY_WORD).
///
/// Each account ID occupies exactly one leaf in the tree, which is identified by its
/// [`AccountId::prefix`]. In other words, account ID prefixes are unique in the blockchain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountTree {
    smt: Smt,
}

impl AccountTree {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new, empty account tree.
    pub fn new() -> Self {
        AccountTree { smt: Smt::new() }
    }

    /// Returns a new [`Smt`] instantiated with the provided entries.
    ///
    /// If the `concurrent` feature of `miden-crypto` is enabled, this function uses a parallel
    /// implementation to process the entries efficiently, otherwise it defaults to the
    /// sequential implementation.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the provided entries contain multiple commitments for the same account ID.
    /// - multiple account IDs share the same prefix.
    pub fn with_entries(
        entries: impl IntoIterator<Item = (AccountId, Digest)>,
    ) -> Result<Self, AccountTreeError> {
        let smt = Smt::with_entries(
            entries
                .into_iter()
                .map(|(id, commitment)| (Self::account_id_to_key(id), Word::from(commitment))),
        )
        .map_err(|err| {
            let MerkleError::DuplicateValuesForIndex(leaf_idx) = err else {
                unreachable!("the only error returned by Smt::with_entries is of this type");
            };

            // SAFETY: Since we only inserted account IDs into the SMT, it is guaranteed that
            // the leaf_idx is a valid Felt as well as a valid account ID prefix.
            AccountTreeError::DuplicateStateCommitments {
                prefix: AccountIdPrefix::new_unchecked(
                    Felt::try_from(leaf_idx).expect("leaf index should be a valid felt"),
                ),
            }
        })?;

        for (leaf_idx, leaf) in smt.leaves() {
            if leaf.num_entries() >= 2 {
                // SAFETY: Since we only inserted account IDs into the SMT, it is guaranteed that
                // the leaf_idx is a valid Felt as well as a valid account ID prefix.
                return Err(AccountTreeError::DuplicateIdPrefix {
                    duplicate_prefix: Some(AccountIdPrefix::new_unchecked(
                        Felt::try_from(leaf_idx.value())
                            .expect("leaf index should be a valid felt"),
                    )),
                });
            }
        }

        Ok(AccountTree { smt })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns an opening of the leaf associated with the `account_id`. This is a proof of the
    /// current state commitment of the given account ID.
    ///
    /// Conceptually, an opening is a Merkle path to the leaf, as well as the leaf itself.
    pub fn open(&self, account_id: AccountId) -> AccountWitness {
        let key = Self::account_id_to_key(account_id);
        AccountWitness::new(self.smt.open(&key))
    }

    /// Returns the current state commitment of the given account ID.
    pub fn get(&self, account_id: AccountId) -> Digest {
        let key = Self::account_id_to_key(account_id);
        Digest::from(self.smt.get_value(&key))
    }

    /// Returns the root of the tree.
    pub fn root(&self) -> Digest {
        self.smt.root()
    }

    /// Returns the SMT key of the given account ID.
    pub(super) fn account_id_to_key(account_id: AccountId) -> Digest {
        Digest::from([Felt::ZERO, Felt::ZERO, account_id.suffix(), account_id.prefix().as_felt()])
    }

    /// Returns the account ID encoded in the given SMT key, if it can be decoded, `None` otherwise.
    pub(super) fn key_to_account_id(key: Digest) -> Option<AccountId> {
        AccountId::try_from([key.as_elements()[3], key.as_elements()[2]]).ok()
    }

    // PUBLIC MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Inserts the state commitment for the given account ID, returning the previous state
    /// commitment associated with that ID.
    ///
    /// This also recomputes all hashes between the leaf (associated with the key) and the root,
    /// updating the root itself.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the prefix of the account ID already exists in the tree.
    pub fn insert(
        &mut self,
        account_id: AccountId,
        state_commitment: Digest,
    ) -> Result<Digest, AccountTreeError> {
        let key = Self::account_id_to_key(account_id);
        let prev_value = Digest::from(self.smt.insert(key, Word::from(state_commitment)));

        // If the leaf of the account ID now has two or more entries, we've inserted a duplicate
        // prefix.
        if self.smt.get_leaf(&key).num_entries() >= 2 {
            return Err(AccountTreeError::DuplicateIdPrefix {
                duplicate_prefix: Some(account_id.prefix()),
            });
        }

        Ok(prev_value)
    }

    // TODO: add api for computing mutations and applying them

    // HELPERS
    // --------------------------------------------------------------------------------------------
}

impl Default for AccountTree {
    fn default() -> Self {
        Self::new()
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
pub(super) mod tests {
    use assert_matches::assert_matches;

    use super::*;
    use crate::{
        account::{AccountStorageMode, AccountType},
        testing::account_id::account_id,
    };

    pub(crate) fn setup_duplicate_prefix_ids() -> [(AccountId, Digest); 2] {
        let id0 = AccountId::try_from(account_id(
            AccountType::FungibleFaucet,
            AccountStorageMode::Public,
            0xaabb_ccdd,
        ))
        .unwrap();
        let id1 = AccountId::try_from(account_id(
            AccountType::FungibleFaucet,
            AccountStorageMode::Public,
            0xaabb_ccff,
        ))
        .unwrap();
        assert_eq!(id0.prefix(), id1.prefix(), "test requires that these ids have the same prefix");

        let commitment0 = Digest::from([Felt::ZERO, Felt::ZERO, Felt::ZERO, Felt::new(42)]);
        let commitment1 = Digest::from([Felt::ZERO, Felt::ZERO, Felt::ZERO, Felt::new(24)]);

        assert_eq!(id0.prefix(), id1.prefix(), "test requires that these ids have the same prefix");
        [(id0, commitment0), (id1, commitment1)]
    }

    #[test]
    fn insert_fails_on_duplicate_prefix() {
        let mut tree = AccountTree::new();
        let [(id0, commitment0), (id1, commitment1)] = setup_duplicate_prefix_ids();

        tree.insert(id0, commitment0).unwrap();
        assert_eq!(tree.get(id0), commitment0);

        let err = tree.insert(id1, commitment1).unwrap_err();

        assert_matches!(err, AccountTreeError::DuplicateIdPrefix {
          duplicate_prefix: Some(prefix)
        } if prefix == id0.prefix());
    }

    #[test]
    fn with_entries_fails_on_duplicate_prefix() {
        let entries = setup_duplicate_prefix_ids();

        let err = AccountTree::with_entries(entries.iter().copied()).unwrap_err();

        assert_matches!(err, AccountTreeError::DuplicateIdPrefix {
          duplicate_prefix: Some(prefix)
        } if prefix == entries[0].0.prefix());
    }

    #[test]
    fn insert_succeeds_on_multiple_updates() {
        let mut tree = AccountTree::new();
        let [(id0, commitment0), (_, commitment1)] = setup_duplicate_prefix_ids();

        tree.insert(id0, commitment0).unwrap();
        tree.insert(id0, commitment1).unwrap();
        assert_eq!(tree.get(id0), commitment1);
    }
}
