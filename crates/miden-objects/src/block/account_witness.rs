use miden_crypto::merkle::MerklePath;
use vm_processor::Digest;

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
    pub fn new(initial_state_commitment: Digest, proof: MerklePath) -> Self {
        Self { initial_state_commitment, proof }
    }

    pub fn initial_state_commitment(&self) -> Digest {
        self.initial_state_commitment
    }

    pub fn proof(&self) -> &MerklePath {
        &self.proof
    }

    pub fn into_parts(self) -> (Digest, MerklePath) {
        (self.initial_state_commitment, self.proof)
    }
}
