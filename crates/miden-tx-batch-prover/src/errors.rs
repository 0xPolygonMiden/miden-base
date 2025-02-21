use miden_objects::transaction::TransactionId;
use miden_tx::TransactionVerifierError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProvenBatchError {
    #[error("failed to verify transaction {transaction_id} in transaction batch")]
    TransactionVerificationFailed {
        transaction_id: TransactionId,
        source: TransactionVerifierError,
    },
}
