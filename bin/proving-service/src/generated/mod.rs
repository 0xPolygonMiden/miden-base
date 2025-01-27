use miden_objects::transaction::ProvenTransaction;
use miden_tx::utils::{Deserializable, DeserializationError, Serializable};

#[rustfmt::skip]
pub mod tx_prover;

pub use tx_prover::*;

// CONVERSIONS
// ================================================================================================

impl From<ProvenTransaction> for ProveTransactionResponse {
    fn from(value: ProvenTransaction) -> Self {
        ProveTransactionResponse { proven_transaction: value.to_bytes() }
    }
}

impl TryFrom<ProveTransactionResponse> for ProvenTransaction {
    type Error = DeserializationError;

    fn try_from(response: ProveTransactionResponse) -> Result<Self, Self::Error> {
        ProvenTransaction::read_from_bytes(&response.proven_transaction)
    }
}
