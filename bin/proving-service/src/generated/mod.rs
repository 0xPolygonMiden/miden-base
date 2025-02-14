use miden_objects::transaction::ProvenTransaction;
use miden_tx::utils::{Deserializable, DeserializationError, Serializable};

#[rustfmt::skip]
pub mod proving_service;

pub use proving_service::*;

// CONVERSIONS
// ================================================================================================

impl From<ProvenTransaction> for ProvingResponse {
    fn from(value: ProvenTransaction) -> Self {
        ProvingResponse { payload: value.to_bytes() }
    }
}

impl TryFrom<ProvingResponse> for ProvenTransaction {
    type Error = DeserializationError;

    fn try_from(response: ProvingResponse) -> Result<Self, Self::Error> {
        ProvenTransaction::read_from_bytes(&response.payload)
    }
}
