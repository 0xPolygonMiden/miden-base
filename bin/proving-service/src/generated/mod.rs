use miden_objects::{
    batch::ProposedBatch,
    block::ProposedBlock,
    transaction::{ProvenTransaction, TransactionWitness},
};
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

impl TryFrom<ProvingRequest> for TransactionWitness {
    type Error = DeserializationError;

    fn try_from(request: ProvingRequest) -> Result<Self, Self::Error> {
        TransactionWitness::read_from_bytes(&request.payload)
    }
}

impl TryFrom<ProvingRequest> for ProposedBatch {
    type Error = DeserializationError;

    fn try_from(request: ProvingRequest) -> Result<Self, Self::Error> {
        ProposedBatch::read_from_bytes(&request.payload)
    }
}

impl TryFrom<ProvingRequest> for ProposedBlock {
    type Error = DeserializationError;

    fn try_from(request: ProvingRequest) -> Result<Self, Self::Error> {
        ProposedBlock::read_from_bytes(&request.payload)
    }
}
