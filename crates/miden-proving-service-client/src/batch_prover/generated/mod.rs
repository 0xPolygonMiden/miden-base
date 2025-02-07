use miden_objects::{batch::ProvenBatch, transaction::ProvenTransaction};
use miden_tx::utils::{Deserializable, DeserializationError, Serializable};

#[cfg(all(feature = "std", target_arch = "wasm32"))]
compile_error!("The `std` feature cannot be used when targeting `wasm32`.");

#[cfg(feature = "std")]
mod std;
#[cfg(feature = "std")]
pub use std::batch_prover::*;

#[cfg(not(feature = "std"))]
mod nostd;
#[cfg(not(feature = "std"))]
pub use nostd::batch_prover::*;

// CONVERSIONS
// ================================================================================================

impl From<ProvenBatch> for ProveBatchResponse {
    fn from(value: ProvenBatch) -> Self {
        ProveBatchResponse { proven_batch: value.to_bytes() }
    }
}

impl TryFrom<ProveBatchResponse> for ProvenBatch {
    type Error = DeserializationError;

    fn try_from(response: ProveBatchResponse) -> Result<Self, Self::Error> {
        ProvenBatch::read_from_bytes(&response.proven_batch)
    }
}
