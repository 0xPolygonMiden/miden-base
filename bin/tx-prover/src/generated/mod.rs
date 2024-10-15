use miden_objects::transaction::ProvenTransaction;
use miden_tx::utils::{Deserializable, DeserializationError, Serializable};

#[cfg(feature = "std")]
mod std;
#[cfg(feature = "std")]
pub use std::api::*;

#[cfg(feature = "wasm-transport")]
mod nostd;
#[cfg(feature = "wasm-transport")]
pub use nostd::api::*;

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
