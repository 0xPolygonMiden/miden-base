use alloc::{boxed::Box, string::ToString};
use core::cell::RefCell;

use miden_objects::transaction::{ProvenTransaction, TransactionWitness};
use miden_tx::{TransactionProver, TransactionProverError};

use crate::{generated::api_client::ApiClient, RemoteTransactionProverError};

// REMOTE TRANSACTION PROVER
// ================================================================================================

/// A [RemoteTransactionProver] is a transaction prover that sends witness data to a remote
/// gRPC server and receives a proven transaction.
#[derive(Clone)]
pub struct RemoteTransactionProver {
    #[cfg(feature = "wasm-transport")]
    client: RefCell<ApiClient<tonic_web_wasm_client::Client>>,
    #[cfg(not(feature = "wasm-transport"))]
    client: RefCell<ApiClient<tonic::transport::Channel>>,
}

impl RemoteTransactionProver {
    /// Creates a new [RemoteTransactionProver] with the specified gRPC server endpoint.
    /// This instantiates a tonic client that attempts connecting with the server.
    ///
    /// When the "std" feature is turned on, this uses the built-in generated tonic transport.
    /// Otherwise, the tonic_web_wasm_client transport is utilized.
    ///
    /// # Errors
    ///
    /// This function will return an error if the endpoint is invalid or if the gRPC
    /// connection to the server cannot be established.
    pub async fn new(endpoint: &str) -> Result<Self, RemoteTransactionProverError> {
        #[cfg(feature = "wasm-transport")]
        let client = {
            let web_client = tonic_web_wasm_client::Client::new(endpoint.to_string());
            ApiClient::new(web_client)
        };
        #[cfg(not(feature = "wasm-transport"))]
        let client = ApiClient::connect(endpoint.to_string())
            .await
            .map_err(|_| RemoteTransactionProverError::ConnectionFailed(endpoint.to_string()))?;

        Ok(RemoteTransactionProver { client: RefCell::new(client) })
    }
}

#[async_trait::async_trait(?Send)]
impl TransactionProver for RemoteTransactionProver {
    async fn prove(
        &self,
        tx_witness: TransactionWitness,
    ) -> Result<ProvenTransaction, TransactionProverError> {
        use miden_objects::utils::Serializable;
        let mut client = self.client.borrow_mut();

        let request = tonic::Request::new(crate::generated::ProveTransactionRequest {
            transaction_witness: tx_witness.to_bytes(),
        });

        let response = client
            .prove_transaction(request)
            .await
            .map_err(|err| TransactionProverError::InternalError(err.to_string()))?;

        // Deserialize the response bytes back into a ProvenTransaction.
        let proven_transaction = ProvenTransaction::try_from(response.into_inner())
            .map_err(TransactionProverError::DeserializationError)?;

        Ok(proven_transaction)
    }
}
