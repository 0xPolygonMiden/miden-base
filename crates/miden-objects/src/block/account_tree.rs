use miden_crypto::merkle::{LeafIndex, MerkleError, MutationSet, Smt, SmtLeaf};
use vm_processor::SMT_DEPTH;

use crate::{
    Digest, Felt, FieldElement, Word,
    account::{AccountId, AccountIdPrefix},
    block::AccountWitness,
    errors::AccountTreeError,
};

// ACCOUNT TREE
// ================================================================================================

/// The sparse merkle tree of all accounts in the blockchain.
///
/// The key is the [`AccountId`] while the value is the current state commitment of the account,
/// i.e. [`Account::commitment`](crate::account::Account::commitment). If the account is new, then
/// the commitment is the [`EMPTY_WORD`](crate::EMPTY_WORD).
///
/// Each account ID occupies exactly one leaf in the tree, which is identified by its
/// [`AccountId::prefix`]. In other words, account ID prefixes are unique in the blockchain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountTree {
    smt: Smt,
}

impl AccountTree {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// The depth of the account tree.
    pub const DEPTH: u8 = SMT_DEPTH;

    /// The index of the account ID suffix in the SMT key.
    pub(super) const KEY_SUFFIX_IDX: usize = 2;
    /// The index of the account ID prefix in the SMT key.
    pub(super) const KEY_PREFIX_IDX: usize = 3;

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
                    duplicate_prefix: AccountIdPrefix::new_unchecked(
                        Felt::try_from(leaf_idx.value())
                            .expect("leaf index should be a valid felt"),
                    ),
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
        let proof = self.smt.open(&key);

        // Check which account ID this proof actually contains. We rely on the fact that the tree
        // only contains zero or one entry per account ID prefix.
        //
        // If the requested account ID matches an existing ID's prefix but their suffixes do not
        // match, then this witness is for the _existing ID_.
        //
        // Otherwise, if the ID matches the one in the leaf or if it's empty, the witness is for the
        // requested ID.
        let witness_id = match proof.leaf() {
            SmtLeaf::Empty(_) => account_id,
            SmtLeaf::Single((key_in_leaf, _)) => {
                // SAFETY: By construction, the tree only contains valid IDs.
                Self::key_to_account_id(*key_in_leaf)
            },
            SmtLeaf::Multiple(_) => {
                unreachable!("account tree should only contain zero or one entry per ID prefix")
            },
        };

        // SAFETY: The tree only contains unique prefixes.
        AccountWitness::new_unchecked(witness_id, proof)
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

    /// Returns the number of account IDs in this tree.
    pub fn num_accounts(&self) -> usize {
        // Because each ID's prefix is unique in the tree and occupies a single leaf, the number of
        // IDs in the tree is equivalent to the number of leaves in the tree.
        self.smt.num_leaves()
    }

    /// Returns an iterator over the account ID state commitment pairs in the tree.
    pub fn account_commitments(&self) -> impl Iterator<Item = (AccountId, Digest)> {
        self.smt.leaves().map(|(_leaf_idx, leaf)| {
            // SAFETY: By construction no Multiple variant is ever present in the tree.
            // The Empty variant is not returned by Smt::leaves, because it only returns leaves that
            // are actually present.
            let SmtLeaf::Single((key, commitment)) = leaf else {
                unreachable!("empty and multiple variant should never be encountered")
            };

            (
                // SAFETY: By construction, the tree only contains valid IDs.
                AccountId::try_from([key[Self::KEY_PREFIX_IDX], key[Self::KEY_SUFFIX_IDX]])
                    .expect("account tree should only contain valid IDs"),
                Digest::from(commitment),
            )
        })
    }

    /// Computes the necessary changes to insert the specified (account ID, state commitment) pairs
    /// into this tree, allowing for validation before applying those changes.
    ///
    /// [`Self::apply_mutations`] can be used in order to commit these changes to the tree.
    ///
    /// If the `concurrent` feature of `miden-crypto` is enabled, this function uses a parallel
    /// implementation to compute the mutations, otherwise it defaults to the sequential
    /// implementation.
    ///
    /// This is a thin wrapper around [`Smt::compute_mutations`]. See its documentation for more
    /// details.
    pub fn compute_mutations(
        &self,
        account_commitments: impl IntoIterator<Item = (AccountId, Digest)>,
    ) -> AccountMutationSet {
        let mutation_set = self.smt.compute_mutations(
            account_commitments
                .into_iter()
                .map(|(id, commitment)| (Self::account_id_to_key(id), Word::from(commitment))),
        );

        AccountMutationSet::new(mutation_set)
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
                duplicate_prefix: account_id.prefix(),
            });
        }

        Ok(prev_value)
    }

    /// Applies the prospective mutations computed with [`Self::compute_mutations`] to this tree.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `mutations` was computed on a tree with a different root than this one.
    pub fn apply_mutations(
        &mut self,
        mutations: AccountMutationSet,
    ) -> Result<(), AccountTreeError> {
        self.smt
            .apply_mutations(mutations.into_mutation_set())
            .map_err(AccountTreeError::ApplyMutations)
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Returns the SMT key of the given account ID.
    pub(super) fn account_id_to_key(account_id: AccountId) -> Digest {
        // We construct this in such a way that we're forced to use the constants, so that when
        // they're updated, the other usages of the constants are also updated.
        let mut key = [Felt::ZERO, Felt::ZERO, Felt::ZERO, Felt::ZERO];
        key[Self::KEY_SUFFIX_IDX] = account_id.suffix();
        key[Self::KEY_PREFIX_IDX] = account_id.prefix().as_felt();

        Digest::from(key)
    }

    /// Returns the [`LeafIndex`] corresponding to the provided [`AccountIdPrefix`].
    pub(super) fn account_id_prefix_to_leaf_index(
        id_prefix: AccountIdPrefix,
    ) -> LeafIndex<{ Self::DEPTH }> {
        LeafIndex::new(id_prefix.as_u64())
            .expect("prefix as u64 should not exceed 2^{AccountTree::DEPTH}")
    }

    /// Returns the [`AccountId`] recovered from the given SMT key.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - the key is not a valid account ID. This should not happen when used on keys from (partial)
    ///   account tree.
    pub(super) fn key_to_account_id(key: Digest) -> AccountId {
        AccountId::try_from([key[Self::KEY_PREFIX_IDX], key[Self::KEY_SUFFIX_IDX]])
            .expect("account tree should only contain valid IDs")
    }
}

impl Default for AccountTree {
    fn default() -> Self {
        Self::new()
    }
}

// ACCOUNT MUTATION SET
// ================================================================================================

/// A newtype wrapper around a [`MutationSet`] which exists for type safety reasons.
///
/// It is returned by and used in methods on the [`AccountTree`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountMutationSet {
    mutation_set: MutationSet<{ AccountTree::DEPTH }, Digest, Word>,
}

impl AccountMutationSet {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`AccountMutationSet`] from the provided raw mutation set.
    fn new(mutation_set: MutationSet<{ AccountTree::DEPTH }, Digest, Word>) -> Self {
        Self { mutation_set }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a reference to the underlying [`MutationSet`].
    pub fn as_mutation_set(&self) -> &MutationSet<{ AccountTree::DEPTH }, Digest, Word> {
        &self.mutation_set
    }

    // PUBLIC MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Consumes self and returns the underlying [`MutationSet`].
    pub fn into_mutation_set(self) -> MutationSet<{ AccountTree::DEPTH }, Digest, Word> {
        self.mutation_set
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
pub(super) mod tests {
    use std::vec::Vec;

    use assert_matches::assert_matches;
    use vm_core::EMPTY_WORD;

    use super::*;
    use crate::{
        account::{AccountStorageMode, AccountType},
        testing::account_id::{AccountIdBuilder, account_id},
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
          duplicate_prefix
        } if duplicate_prefix == id0.prefix());
    }

    #[test]
    fn with_entries_fails_on_duplicate_prefix() {
        let entries = setup_duplicate_prefix_ids();

        let err = AccountTree::with_entries(entries.iter().copied()).unwrap_err();

        assert_matches!(err, AccountTreeError::DuplicateIdPrefix {
          duplicate_prefix
        } if duplicate_prefix == entries[0].0.prefix());
    }

    #[test]
    fn insert_succeeds_on_multiple_updates() {
        let mut tree = AccountTree::new();
        let [(id0, commitment0), (_, commitment1)] = setup_duplicate_prefix_ids();

        tree.insert(id0, commitment0).unwrap();
        tree.insert(id0, commitment1).unwrap();
        assert_eq!(tree.get(id0), commitment1);
    }

    #[test]
    fn apply_mutations() {
        let id0 = AccountIdBuilder::new().build_with_seed([5; 32]);
        let id1 = AccountIdBuilder::new().build_with_seed([6; 32]);
        let id2 = AccountIdBuilder::new().build_with_seed([7; 32]);

        let digest0 = Digest::from([0, 0, 0, 1u32]);
        let digest1 = Digest::from([0, 0, 0, 2u32]);
        let digest2 = Digest::from([0, 0, 0, 3u32]);
        let digest3 = Digest::from([0, 0, 0, 4u32]);

        let mut tree = AccountTree::with_entries([(id0, digest0), (id1, digest1)]).unwrap();

        let mutations = tree.compute_mutations([(id0, digest1), (id1, digest2), (id2, digest3)]);

        tree.apply_mutations(mutations).unwrap();

        assert_eq!(tree.num_accounts(), 3);
        assert_eq!(tree.get(id0), digest1);
        assert_eq!(tree.get(id1), digest2);
        assert_eq!(tree.get(id2), digest3);
    }

    #[test]
    fn account_commitments() {
        let id0 = AccountIdBuilder::new().build_with_seed([5; 32]);
        let id1 = AccountIdBuilder::new().build_with_seed([6; 32]);
        let id2 = AccountIdBuilder::new().build_with_seed([7; 32]);

        let digest0 = Digest::from([0, 0, 0, 1u32]);
        let digest1 = Digest::from([0, 0, 0, 2u32]);
        let digest2 = Digest::from([0, 0, 0, 3u32]);
        let empty_digest = Digest::from(EMPTY_WORD);

        let mut tree =
            AccountTree::with_entries([(id0, digest0), (id1, digest1), (id2, digest2)]).unwrap();

        // remove id2
        tree.insert(id2, empty_digest).unwrap();

        assert_eq!(tree.num_accounts(), 2);

        let accounts: Vec<_> = tree.account_commitments().collect();
        assert_eq!(accounts.len(), 2);
        assert!(accounts.contains(&(id0, digest0)));
        assert!(accounts.contains(&(id1, digest1)));
    }

    #[test]
    fn account_witness() {
        let id0 = AccountIdBuilder::new().build_with_seed([5; 32]);
        let id1 = AccountIdBuilder::new().build_with_seed([6; 32]);

        let digest0 = Digest::from([0, 0, 0, 1u32]);
        let digest1 = Digest::from([0, 0, 0, 2u32]);

        let tree = AccountTree::with_entries([(id0, digest0), (id1, digest1)]).unwrap();

        assert_eq!(tree.num_accounts(), 2);

        for id in [id0, id1] {
            let (control_path, control_leaf) =
                tree.smt.open(&AccountTree::account_id_to_key(id)).into_parts();
            let witness = tree.open(id);

            assert_eq!(witness.leaf(), control_leaf);
            assert_eq!(witness.path(), &control_path);
        }
    }
}
