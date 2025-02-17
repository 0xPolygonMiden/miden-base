use miden_objects::{
    batch::{ProposedBatch, ProvenBatch},
    transaction::{ProvenTransaction, TransactionWitness},
};
use miden_tx::utils::{Deserializable, DeserializationError, Serializable};

#[cfg(all(feature = "std", target_arch = "wasm32"))]
compile_error!("The `std` feature cannot be used when targeting `wasm32`.");

#[cfg(feature = "std")]
mod std;
#[cfg(feature = "std")]
pub use std::remote_prover::*;

#[cfg(not(feature = "std"))]
mod nostd;
#[cfg(not(feature = "std"))]
pub use nostd::remote_prover::*;

// CONVERSIONS
// ================================================================================================

impl From<ProvenTransaction> for ProveResponse {
    fn from(value: ProvenTransaction) -> Self {
        ProveResponse { payload: value.to_bytes() }
    }
}

impl TryFrom<ProveResponse> for ProvenTransaction {
    type Error = DeserializationError;

    fn try_from(response: ProveResponse) -> Result<Self, Self::Error> {
        ProvenTransaction::read_from_bytes(&response.payload)
    }
}

impl From<TransactionWitness> for ProveRequest {
    fn from(witness: TransactionWitness) -> Self {
        ProveRequest {
            proof_type: 0,
            payload: witness.to_bytes(),
        }
    }
}

impl From<ProposedBatch> for ProveRequest {
    fn from(proposed_batch: ProposedBatch) -> Self {
        ProveRequest {
            proof_type: 1,
            payload: proposed_batch.to_bytes(),
        }
    }
}

impl TryFrom<ProveResponse> for ProvenBatch {
    type Error = DeserializationError;

    fn try_from(response: ProveResponse) -> Result<Self, Self::Error> {
        ProvenBatch::read_from_bytes(&response.payload)
    }
}
