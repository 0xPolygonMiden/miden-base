pub mod generated;

use alloc::string::String;

use miden_objects::batch::{ProposedBatch, ProvenBatch};
use miden_tx::utils::sync::RwLock;
use miden_tx_batch_prover::errors::BatchProveError;

use crate::{
    alloc::string::ToString,
    batch_prover::generated::{api_client::ApiClient, ProveBatchRequest},
    RemoteProverError,
};

/// Protobuf definition for the Miden proving service
pub const BATCH_PROVER_PROTO: &str = include_str!("../../proto/batch_prover.proto");

// REMOTE BATCH PROVER
// ================================================================================================

/// A [RemoteBatchProver] is a batch prover that sends a proposed batch to a remote
/// gRPC server and receives a proven batch.
///
/// When compiled for the `wasm32-unknown-unknown` target, it uses the `tonic_web_wasm_client`
/// transport. Otherwise, it uses the built-in `tonic::transport` for native platforms.
///
/// The transport layer connection is established lazily when the first batch is proven.
pub struct RemoteBatchProver {
    #[cfg(target_arch = "wasm32")]
    client: RwLock<Option<ApiClient<tonic_web_wasm_client::Client>>>,

    #[cfg(not(target_arch = "wasm32"))]
    client: RwLock<Option<ApiClient<tonic::transport::Channel>>>,

    endpoint: String,
}

impl RemoteBatchProver {
    /// Creates a new [RemoteBatchProver] with the specified gRPC server endpoint. The
    /// endpoint should be in the format `{protocol}://{hostname}:{port}`.
    pub fn new(endpoint: &str) -> Self {
        RemoteBatchProver {
            endpoint: endpoint.to_string(),
            client: RwLock::new(None),
        }
    }

    /// Establishes a connection to the remote batch prover server. The connection is
    /// maintained for the lifetime of the prover. If the connection is already established, this
    /// method does nothing.
    async fn connect(&self) -> Result<(), RemoteProverError> {
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
            .write()
            .as_ref()
            .ok_or_else(|| BatchProveError::other("client should be connected"))?
            .clone();

        let request = tonic::Request::new(generated::ProveBatchRequest {
            proposed_batch: proposed_batch.to_bytes(),
        });

        let response = client
            .prove_batch(request)
            .await
            .map_err(|err| BatchProveError::other_with_source("failed to prove batch", err))?;

        // Deserialize the response bytes back into a ProvenBatch.
        let proven_batch = ProvenBatch::try_from(response.into_inner()).map_err(|_| {
            BatchProveError::other(
                "failed to deserialize received response from remote batch prover",
            )
        })?;

        Ok(proven_batch)
    }
}
