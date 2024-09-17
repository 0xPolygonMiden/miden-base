use miden_objects::transaction::ProvenTransaction;
use miden_tx::utils::Serializable;

use crate::ProveTransactionResponse;

impl From<ProvenTransaction> for ProveTransactionResponse {
    fn from(value: ProvenTransaction) -> Self {
        ProveTransactionResponse { proven_transaction: value.to_bytes() }
    }
}
