use miden_objects::transaction::ProvenTransaction;
use miden_tx::utils::{Deserializable, DeserializationError, Serializable};

#[rustfmt::skip]
pub mod remote_prover;

pub use remote_prover::*;

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
