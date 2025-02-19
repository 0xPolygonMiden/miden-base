use miden_objects::{
    batch::{ProposedBatch, ProvenBatch},
    block::{ProposedBlock, ProvenBlock},
    transaction::{ProvenTransaction, TransactionWitness},
};
use miden_tx::utils::{Deserializable, DeserializationError, Serializable};

#[cfg(all(feature = "std", target_arch = "wasm32"))]
compile_error!("The `std` feature cannot be used when targeting `wasm32`.");

#[cfg(feature = "std")]
mod std;
#[cfg(feature = "std")]
pub use std::proving_service::*;

#[cfg(not(feature = "std"))]
mod nostd;
#[cfg(not(feature = "std"))]
pub use nostd::proving_service::*;

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

impl From<TransactionWitness> for ProvingRequest {
    fn from(witness: TransactionWitness) -> Self {
        ProvingRequest {
            proof_type: ProofType::Transaction.into(),
            payload: witness.to_bytes(),
        }
    }
}

impl From<ProposedBatch> for ProvingRequest {
    fn from(proposed_batch: ProposedBatch) -> Self {
        ProvingRequest {
            proof_type: ProofType::Batch.into(),
            payload: proposed_batch.to_bytes(),
        }
    }
}

impl TryFrom<ProvingResponse> for ProvenBatch {
    type Error = DeserializationError;

    fn try_from(response: ProvingResponse) -> Result<Self, Self::Error> {
        ProvenBatch::read_from_bytes(&response.payload)
    }
}

impl TryFrom<ProvingResponse> for ProvenBlock {
    type Error = DeserializationError;

    fn try_from(value: ProvingResponse) -> Result<Self, Self::Error> {
        ProvenBlock::read_from_bytes(&value.payload)
    }
}

impl From<ProposedBlock> for ProvingRequest {
    fn from(proposed_block: ProposedBlock) -> Self {
        ProvingRequest {
            proof_type: ProofType::Block.into(),
            payload: proposed_block.to_bytes(),
        }
    }
}
