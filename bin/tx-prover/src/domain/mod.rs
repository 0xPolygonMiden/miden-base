use miden_objects::transaction::ProvenTransaction;
use miden_tx::{utils::{Deserializable, Serializable}, TransactionProverError};

use crate::ProveTransactionResponse;

impl From<ProvenTransaction> for ProveTransactionResponse {
    fn from(value: ProvenTransaction) -> Self {
        ProveTransactionResponse { proven_transaction: value.to_bytes() }
    }
}

impl TryFrom<ProveTransactionResponse> for ProvenTransaction {
    type Error = TransactionProverError;

    fn try_from(response: ProveTransactionResponse) -> Result<Self, Self::Error> {
        ProvenTransaction::read_from_bytes(&response.proven_transaction)
            .map_err(|_err| TransactionProverError::DeserializationError)
    }
}
