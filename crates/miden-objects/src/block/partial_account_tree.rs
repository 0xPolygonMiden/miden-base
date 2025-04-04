use crate::{
    Digest, Word,
    account::AccountId,
    block::{AccountTree, AccountWitness},
    crypto::merkle::PartialSmt,
    errors::AccountTreeError,
};

/// The partial sparse merkle tree containing the state commitments of accounts in the chain.
///
/// This is the partial version of [`AccountTree`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialAccountTree {
    smt: PartialSmt,
}

impl PartialAccountTree {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new, empty partial account tree.
    pub fn new() -> Self {
        PartialAccountTree { smt: PartialSmt::new() }
    }

    /// Returns a new [`PartialAccountTree`] instantiated with the provided entries.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the provided entries contain multiple commitments for the same account ID.
    /// - multiple account IDs share the same prefix.
    pub fn with_witnesses(
        witnesses: impl IntoIterator<Item = AccountWitness>,
    ) -> Result<Self, AccountTreeError> {
        let mut tree = Self::new();

        for witness in witnesses {
            tree.track_account_witness(witness)?;
        }

        Ok(tree)
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns an opening of the leaf associated with the `account_id`. This is a proof of the
    /// current state commitment of the given account ID.
    ///
    /// Conceptually, an opening is a Merkle path to the leaf, as well as the leaf itself.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the account ID is not tracked by this account tree.
    pub fn open(&self, account_id: AccountId) -> Result<AccountWitness, AccountTreeError> {
        let key = AccountTree::account_id_to_key(account_id);
        // SAFETY: The tree only contains unique prefixes.
        self.smt
            .open(&key)
            .map(AccountWitness::new_unchecked)
            .map_err(|source| AccountTreeError::UntrackedAccountId { id: account_id, source })
    }

    /// Returns the current state commitment of the given account ID.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the account ID is not tracked by this account tree.
    pub fn get(&self, account_id: AccountId) -> Result<Digest, AccountTreeError> {
        let key = AccountTree::account_id_to_key(account_id);
        self.smt
            .get_value(&key)
            .map(Digest::from)
            .map_err(|source| AccountTreeError::UntrackedAccountId { id: account_id, source })
    }

    /// Returns the root of the tree.
    pub fn root(&self) -> Digest {
        self.smt.root()
    }

    // PUBLIC MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Adds the given account witness to the partial tree and tracks it. Once an account has
    /// been added to the tree, it can be updated using [`Self::upsert_state_commitments`].
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - after the witness' merkle path was added, the partial account tree has a different root
    ///   than before it was added.
    /// - the prefix of the account ID tracked by the witness is already in the tree.
    pub fn track_account_witness(
        &mut self,
        witness: AccountWitness,
    ) -> Result<(), AccountTreeError> {
        let id_prefix = witness.id_prefix();
        let (path, leaf) = witness.into_proof().into_parts();
        if leaf.is_empty() {
            return Ok(());
        }

        let id_key = leaf.entries().first().expect("there should be at least one entry").0;

        self.smt.add_path(leaf, path).map_err(AccountTreeError::TreeRootConflict)?;

        if self
            .smt
            .get_leaf(&id_key)
            .expect("the key should be tracked because we just inserted it")
            .num_entries()
            >= 2
        {
            return Err(AccountTreeError::DuplicateIdPrefix { duplicate_prefix: id_prefix });
        };

        Ok(())
    }

    /// Inserts or updates the provided account ID -> state commitment updates into the partial tree
    /// which results in a new tree root.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the prefix of the account ID already exists in the tree.
    /// - the account_id is not tracked by this partial account tree.
    pub fn upsert_state_commitments(
        &mut self,
        updates: impl IntoIterator<Item = (AccountId, Digest)>,
    ) -> Result<(), AccountTreeError> {
        for (account_id, state_commitment) in updates {
            self.insert(account_id, state_commitment)?;
        }

        Ok(())
    }

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
    /// - the account_id is not tracked by this partial account tree.
    fn insert(
        &mut self,
        account_id: AccountId,
        state_commitment: Digest,
    ) -> Result<Digest, AccountTreeError> {
        let key = AccountTree::account_id_to_key(account_id);
        let prev_value =
            self.smt.insert(key, Word::from(state_commitment)).map(Digest::from).map_err(
                |source| AccountTreeError::UntrackedAccountId { id: account_id, source },
            )?;

        // If the leaf of the account ID _after the insertion_ has two or more entries, we've
        // inserted a duplicate prefix.
        // We check this after the insertion because it is easier to do than before. The downside is
        // that the tree is left in an inconsistent state, but trees on which operations fail are
        // usually discarded anyway.
        if self
            .smt
            .get_leaf(&key)
            .map_err(|source| AccountTreeError::UntrackedAccountId { id: account_id, source })?
            .num_entries()
            >= 2
        {
            return Err(AccountTreeError::DuplicateIdPrefix {
                duplicate_prefix: account_id.prefix(),
            });
        }

        Ok(prev_value)
    }
}

impl Default for PartialAccountTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;

    use super::*;
    use crate::block::account_tree::tests::setup_duplicate_prefix_ids;

    #[test]
    fn insert_fails_on_duplicate_prefix() {
        let mut full_tree = AccountTree::new();
        let mut partial_tree = PartialAccountTree::new();

        let [(id0, commitment0), (id1, commitment1)] = setup_duplicate_prefix_ids();

        full_tree.insert(id0, commitment0).unwrap();
        let witness = full_tree.open(id0);

        partial_tree.track_account_witness(witness).unwrap();

        partial_tree.insert(id0, commitment0).unwrap();
        assert_eq!(partial_tree.get(id0).unwrap(), commitment0);

        let err = partial_tree.insert(id1, commitment1).unwrap_err();

        assert_matches!(err, AccountTreeError::DuplicateIdPrefix {
          duplicate_prefix
        } if duplicate_prefix == id0.prefix());
    }

    #[test]
    fn insert_succeeds_on_multiple_updates() {
        let mut full_tree = AccountTree::new();
        let mut partial_tree = PartialAccountTree::new();
        let [(id0, commitment0), (_, commitment1)] = setup_duplicate_prefix_ids();

        full_tree.insert(id0, commitment0).unwrap();
        let witness = full_tree.open(id0);

        partial_tree.track_account_witness(witness).unwrap();

        partial_tree.insert(id0, commitment0).unwrap();
        partial_tree.insert(id0, commitment1).unwrap();
        assert_eq!(partial_tree.get(id0).unwrap(), commitment1);
    }

    // TODO: Add tests for the errors of upsert_state_commitments.
}
