use alloc::{
    boxed::Box,
    string::{String, ToString},
};

use miden_objects::transaction::{ProvenTransaction, TransactionWitness};
use miden_tx::{utils::sync::RwLock, TransactionProver, TransactionProverError};

use crate::{generated::api_client::ApiClient, RemoteTransactionProverError};

// REMOTE TRANSACTION PROVER
// ================================================================================================

/// A [RemoteTransactionProver] is a transaction prover that sends witness data to a remote
/// gRPC server and receives a proven transaction.
///
/// When compiled for the `wasm32-unknown-unknown` target, it uses the `tonic_web_wasm_client`
/// transport. Otherwise, it uses the built-in `tonic::transport` for native platforms.
///
/// The transport layer connection is established lazily when the first transaction is proven.
pub struct RemoteTransactionProver {
    #[cfg(target_arch = "wasm32")]
    client: RwLock<Option<ApiClient<tonic_web_wasm_client::Client>>>,

    #[cfg(not(target_arch = "wasm32"))]
    client: RwLock<Option<ApiClient<tonic::transport::Channel>>>,

    endpoint: String,
}

impl RemoteTransactionProver {
    /// Creates a new [RemoteTransactionProver] with the specified gRPC server endpoint. The
    /// endpoint should be in the format `{protocol}://{hostname}:{port}`.
    pub fn new(endpoint: &str) -> Self {
        RemoteTransactionProver {
            endpoint: endpoint.to_string(),
            client: RwLock::new(None),
        }
    }

    /// Establishes a connection to the remote transaction prover server. The connection is
    /// mantained for the lifetime of the prover. If the connection is already established, this
    /// method does nothing.
    async fn connect(&self) -> Result<(), RemoteTransactionProverError> {
        let mut client = self.client.write();
        if client.is_some() {
            return Ok(());
        }

        #[cfg(target_arch = "wasm32")]
        let new_client = {
            let web_client = tonic_web_wasm_client::Client::new(self.endpoint.clone());
            ApiClient::new(web_client)
        };

        #[cfg(not(target_arch = "wasm32"))]
        let new_client = {
            ApiClient::connect(self.endpoint.clone()).await.map_err(|_| {
                RemoteTransactionProverError::ConnectionFailed(self.endpoint.to_string())
            })?
        };

        *client = Some(new_client);

        Ok(())
    }
}

#[async_trait::async_trait(?Send)]
impl TransactionProver for RemoteTransactionProver {
    async fn prove(
        &self,
        tx_witness: TransactionWitness,
    ) -> Result<ProvenTransaction, TransactionProverError> {
        use miden_objects::utils::Serializable;
        self.connect().await.map_err(|err| {
            TransactionProverError::other_with_source("failed to connect to the remote prover", err)
        })?;

        let mut client = self.client.write();

        let request = tonic::Request::new(crate::generated::ProveTransactionRequest {
            transaction_witness: tx_witness.to_bytes(),
        });

        let response = client
            .as_mut()
            .expect("client should be connected")
            .prove_transaction(request)
            .await
            .map_err(|err| {
                TransactionProverError::other_with_source("failed to prove transaction", err)
            })?;

        // Deserialize the response bytes back into a ProvenTransaction.
        let proven_transaction =
            ProvenTransaction::try_from(response.into_inner()).map_err(|_| {
                TransactionProverError::other(
                    "failed to deserialize received response from remote transaction prover",
                )
            })?;

        Ok(proven_transaction)
    }
}
