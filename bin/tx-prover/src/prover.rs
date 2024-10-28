use alloc::{
    boxed::Box,
    string::{String, ToString},
};

use miden_objects::transaction::{ProvenTransaction, TransactionWitness};
use miden_tx::{TransactionProver, TransactionProverError};

use crate::generated::api_client::ApiClient;

// REMOTE TRANSACTION PROVER
// ================================================================================================

/// A [RemoteTransactionProver] is a transaction prover that sends witness data to a remote
/// gRPC server and receives a proven transaction.
///
/// When compiled for the `wasm32-unknown-unknown` target, it uses the `tonic_web_wasm_client`
/// transport. Otherwise, it uses the built-in `tonic::transport` for native platforms.
/// The transport layer connection is established lazily when the first transaction is proven.

#[derive(Clone)]
pub struct RemoteTransactionProver {
    endpoint: String,
}

impl RemoteTransactionProver {
    /// Creates a new [RemoteTransactionProver] with the specified gRPC server endpoint. The
    /// endpoint should be in the format `{protocol}://{hostname}:{port}`.
    pub fn new(endpoint: &str) -> Self {
        RemoteTransactionProver { endpoint: endpoint.to_string() }
    }
}

#[async_trait::async_trait(?Send)]
impl TransactionProver for RemoteTransactionProver {
    async fn prove(
        &self,
        tx_witness: TransactionWitness,
    ) -> Result<ProvenTransaction, TransactionProverError> {
        use miden_objects::utils::Serializable;
        #[cfg(target_arch = "wasm32")]
        let mut client = {
            let web_client = tonic_web_wasm_client::Client::new(self.endpoint.clone());
            ApiClient::new(web_client)
        };

        #[cfg(not(target_arch = "wasm32"))]
        let mut client = ApiClient::connect(self.endpoint.clone()).await.map_err(|_| {
            TransactionProverError::InternalError(format!(
                "Failed to connect to endpoint {}",
                self.endpoint
            ))
        })?;

        let request = tonic::Request::new(crate::generated::ProveTransactionRequest {
            transaction_witness: tx_witness.to_bytes(),
        });

        let response = client
            .prove_transaction(request)
            .await
            .map_err(|err| TransactionProverError::InternalError(err.to_string()))?;

        // Deserialize the response bytes back into a ProvenTransaction.
        let proven_transaction =
            ProvenTransaction::try_from(response.into_inner()).map_err(|_| {
                TransactionProverError::InternalError(
                    "Error deserializing received response".to_string(),
                )
            })?;

        Ok(proven_transaction)
    }
}
