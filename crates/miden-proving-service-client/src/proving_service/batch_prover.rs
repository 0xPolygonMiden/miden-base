use alloc::{
    string::{String, ToString},
    sync::Arc,
};

use miden_objects::{
    batch::{ProposedBatch, ProvenBatch},
    utils::{Deserializable, DeserializationError, Serializable},
};
use tokio::sync::Mutex;

use super::generated::api_client::ApiClient;
use crate::{
    proving_service::generated::{ProofType, ProvingRequest, ProvingResponse},
    RemoteProverError,
};

// REMOTE BATCH PROVER
// ================================================================================================

/// A [`RemoteBatchProver`] is a batch prover that sends a proposed batch data to a remote
/// gRPC server and receives a proven batch.
///
/// When compiled for the `wasm32-unknown-unknown` target, it uses the `tonic_web_wasm_client`
/// transport. Otherwise, it uses the built-in `tonic::transport` for native platforms.
///
/// The transport layer connection is established lazily when the first batch is proven.
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

    /// Establishes a connection to the remote batch prover server. The connection is
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
    ) -> Result<ProvenBatch, RemoteProverError> {
        use miden_objects::utils::Serializable;
        self.connect().await?;

        let mut client = self
            .client
            .lock()
            .await
            .as_ref()
            .ok_or_else(|| {
                RemoteProverError::ConnectionFailed("client should be connected".into())
            })?
            .clone();

        let request = tonic::Request::new(proposed_batch.into());

        let response = client
            .prove(request)
            .await
            .map_err(|err| RemoteProverError::other_with_source("failed to prove block", err))?;

        // Deserialize the response bytes back into a ProvenBatch.
        let proven_batch = ProvenBatch::try_from(response.into_inner()).map_err(|err| {
            RemoteProverError::other_with_source(
                "failed to deserialize received response from remote prover",
                err,
            )
        })?;

        Ok(proven_batch)
    }
}

// CONVERSIONS
// ================================================================================================

impl From<ProposedBatch> for ProvingRequest {
    fn from(proposed_batch: ProposedBatch) -> Self {
        ProvingRequest {
            proof_type: ProofType::Batch.into(),
            payload: proposed_batch.to_bytes(),
        }
    }
}

impl TryFrom<ProvingResponse> for ProvenBatch {
    type Error = DeserializationError;

    fn try_from(response: ProvingResponse) -> Result<Self, Self::Error> {
        ProvenBatch::read_from_bytes(&response.payload)
    }
}
