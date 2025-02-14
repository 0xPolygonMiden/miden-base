use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
};

use miden_objects::batch::{ProposedBatch, ProvenBatch};
use miden_tx_batch_prover::errors::BatchProveError;
use tokio::sync::Mutex;

use super::generated::api_client::ApiClient;
use crate::RemoteProverError;

// REMOTE BATCH PROVER
// ================================================================================================

/// A [`RemoteBatchProver`] is a batch prover that sends a proposed batch data to a remote
/// gRPC server and receives a proven batch.
///
/// When compiled for the `wasm32-unknown-unknown` target, it uses the `tonic_web_wasm_client`
/// transport. Otherwise, it uses the built-in `tonic::transport` for native platforms.
///
/// The transport layer connection is established lazily when the first transaction is proven.
pub struct RemoteBatchProver {
    #[cfg(target_arch = "wasm32")]
    client: Arc<Mutex<Option<ApiClient<tonic_web_wasm_client::Client>>>>,

    #[cfg(not(target_arch = "wasm32"))]
    client: Arc<Mutex<Option<ApiClient<tonic::transport::Channel>>>>,

    endpoint: String,
}

impl RemoteBatchProver {
    /// Creates a new [RemoteBatchProver] with the specified gRPC server endpoint. The
    /// endpoint should be in the format `{protocol}://{hostname}:{port}`.
    pub fn new(endpoint: impl Into<String>) -> Self {
        RemoteBatchProver {
            endpoint: endpoint.into(),
            client: Arc::new(Mutex::new(None)),
        }
    }

    /// Establishes a connection to the remote transaction prover server. The connection is
    /// maintained for the lifetime of the prover. If the connection is already established, this
    /// method does nothing.
    async fn connect(&self) -> Result<(), RemoteProverError> {
        let mut client = self.client.lock().await;
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
            ApiClient::connect(self.endpoint.clone())
                .await
                .map_err(|_| RemoteProverError::ConnectionFailed(self.endpoint.to_string()))?
        };

        *client = Some(new_client);

        Ok(())
    }
}

impl RemoteBatchProver {
    pub async fn prove(
        &self,
        proposed_batch: ProposedBatch,
    ) -> Result<ProvenBatch, BatchProveError> {
        use miden_objects::utils::Serializable;
        self.connect().await.map_err(|err| {
            BatchProveError::other_with_source("failed to connect to the remote prover", err)
        })?;

        let mut client = self
            .client
            .lock()
            .await
            .as_ref()
            .ok_or_else(|| BatchProveError::other("client should be connected"))?
            .clone();

        let request = tonic::Request::new(proposed_batch.into());

        let response = client.prove(request).await.map_err(|err| {
            BatchProveError::other_with_source("failed to prove transaction", err)
        })?;

        // Deserialize the response bytes back into a ProvenTransaction.
        let proven_transaction = ProvenBatch::try_from(response.into_inner()).map_err(|_| {
            BatchProveError::other(
                "failed to deserialize received response from remote transaction prover",
            )
        })?;

        Ok(proven_transaction)
    }
}
