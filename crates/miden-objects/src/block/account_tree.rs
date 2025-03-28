use miden_crypto::merkle::Smt;

use crate::{Digest, Felt, FieldElement, Word, account::AccountId, block::AccountWitness};

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

    // TODO: with_entries

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns an opening of the leaf associated with the `account_id`.
    ///
    /// Conceptually, an opening is a Merkle path to the leaf, as well as the leaf itself.
    pub fn open(&self, account_id: AccountId) -> AccountWitness {
        let key = Self::account_id_to_key(account_id);
        AccountWitness::new(self.smt.open(&key))
    }

    /// Returns the root of the tree.
    pub fn root(&self) -> Digest {
        self.smt.root()
    }

    /// Returns the SMT key of the given account ID.
    pub(super) fn account_id_to_key(account_id: AccountId) -> Digest {
        Digest::from([Felt::ZERO, Felt::ZERO, account_id.suffix(), account_id.prefix().as_felt()])
    }

    // PUBLIC MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Inserts the state commitment for the given account ID, returning the previous state
    /// commitment associated with that ID.
    ///
    /// This also recomputes all hashes between the leaf (associated with the key) and the root,
    /// updating the root itself.
    pub fn insert(&mut self, account_id: AccountId, state_commitment: Digest) -> Digest {
        let key = Self::account_id_to_key(account_id);
        self.smt.insert(key, Word::from(state_commitment)).into()
    }

    // TODO: add api that makes use of concurrent mutation

    // HELPERS
    // --------------------------------------------------------------------------------------------
}

impl Default for AccountTree {
    fn default() -> Self {
        Self::new()
    }
}
