use miden_crypto::merkle::{SmtLeaf, SmtProof};

use crate::Digest;

// ACCOUNT WITNESS
// ================================================================================================

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
    pub fn state_commitment(&self) -> Digest {
        match self.proof.leaf() {
            SmtLeaf::Empty(_) => Digest::default(),
            SmtLeaf::Single((_, commitment)) => Digest::from(commitment),
            SmtLeaf::Multiple(_) => {
                // SAFETY: The (partial) account tree ensures that it only contains unique account
                // ID prefixes, and so there will never be an smt leaf multiple
                // variant.
                unreachable!("account witness is guaranteed to contain zero or one entries")
            },
        }
    }

    /// Consumes self and returns the inner proof.
    pub fn into_proof(self) -> SmtProof {
        self.proof
    }
}
