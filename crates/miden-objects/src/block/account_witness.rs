use miden_crypto::merkle::SmtProof;

use crate::{Digest, account::AccountId, block::AccountTree};

// ACCOUNT WITNESS
// ================================================================================================

// TODO: Make it a guarantee of the type that it only contains a leaf which is empty or with one
// entry, then we don't need get state commitment to take an account ID.

/// A wrapper around an [`SmtProof`] that proves the inclusion of an account ID at a certain state
/// (i.e. [`Account::commitment`](crate::account::Account::commitment)) in the
/// [`AccountTree`](crate::block::AccountTree).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountWitness {
    proof: SmtProof,
}

impl AccountWitness {
    /// Constructs a new [`AccountWitness`] from the provided proof.
    pub fn new(proof: SmtProof) -> Self {
        Self { proof }
    }

    /// Returns the inner proof for the account tree of this witness.
    pub fn as_proof(&self) -> &SmtProof {
        &self.proof
    }

    /// Returns the state commitment of the given `account_id` if it is in this proof, `None`
    /// otherwise.
    pub fn get_state_commitment(&self, account_id: AccountId) -> Option<Digest> {
        let key = AccountTree::account_id_to_key(account_id);
        self.proof.get(&key).map(Digest::from)
    }

    /// Consumes self and returns the inner proof.
    pub fn into_proof(self) -> SmtProof {
        self.proof
    }
}
