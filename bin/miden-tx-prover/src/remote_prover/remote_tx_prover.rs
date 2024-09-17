use std::sync::Arc;
use miden_objects::transaction::{ProvenTransaction, TransactionWitness};
use miden_tx::{
    utils::{Deserializable, Serializable},
    TransactionProver, TransactionProverError,
};
use tokio::sync::Mutex;

use crate::{ProveTransactionRequest, ProveTransactionResponse};

#[derive(Clone)]
pub struct RemoteTransactionProver {
    pub url: String,
    pub client: Arc<Mutex<reqwest::Client>>,
}

impl RemoteTransactionProver {
    pub fn new(url: String) -> Self {
        Self {
            url,
            client: Arc::new(Mutex::new(reqwest::Client::new())),
        }
    }

    pub(crate) async fn prove(
        &self,
        transaction: impl Into<TransactionWitness>,
    ) -> Result<ProvenTransaction, TransactionProverError> {
        let tx_witness: TransactionWitness = transaction.into();

        let tx_witness_request = ProveTransactionRequest {
            transaction_witness: tx_witness.to_bytes(),
        };

        // Send the POST request
        let client = self.client.lock().await;
        let response = client
            .post(&format!("{}/prove", self.url))
            .header("Content-Type", "application/json")
            .body(tx_witness_request.into())
            .send()
            .await
            .map_err(|_| TransactionProverError::HttpRequestError)?;

        // Check if the response status is success
        if response.status().is_success() {
            let ProveTransactionResponse { proven_transaction } =
                response.try_into().map_err(|_| TransactionProverError::DeserializationError)?;

            Ok(ProvenTransaction::read_from_bytes(&proven_transaction).unwrap())
        } else {
            Err(TransactionProverError::HttpRequestError)
        }
    }
}

impl TransactionProver for RemoteTransactionProver {
    async fn prove(
        &self,
        transaction: impl Into<TransactionWitness>,
    ) -> Result<ProvenTransaction, TransactionProverError> {
        self.prove(transaction).await
    }
}
