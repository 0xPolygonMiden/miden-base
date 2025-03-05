use crate::{crypto::merkle::MerklePath, Digest};

// ACCOUNT WITNESS
// ================================================================================================

/// A proof that a certain account is in the account tree and whose current state is the contained
/// initial state commitment.
#[derive(Debug, Clone)]
pub struct AccountWitness {
    initial_state_commitment: Digest,
    proof: MerklePath,
}

impl AccountWitness {
    /// Constructs a new [`AccountWitness`] from the provided parts.
    pub fn new(initial_state_commitment: Digest, proof: MerklePath) -> Self {
        Self { initial_state_commitment, proof }
    }

    /// Returns the initial state commitment that this witness proves is the current state.
    pub fn initial_state_commitment(&self) -> Digest {
        self.initial_state_commitment
    }

    /// Returns the merkle path for the account tree of this witness.
    pub fn proof(&self) -> &MerklePath {
        &self.proof
    }

    /// Consumes self and returns the parts of the witness.
    pub fn into_parts(self) -> (Digest, MerklePath) {
        (self.initial_state_commitment, self.proof)
    }
}
