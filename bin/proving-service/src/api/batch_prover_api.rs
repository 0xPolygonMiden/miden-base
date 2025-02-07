use miden_objects::{batch::ProposedBatch, MIN_PROOF_SECURITY_LEVEL};
use miden_tx::utils::{Deserializable, Serializable};
use miden_tx_batch_prover::LocalBatchProver;
use tokio::{net::TcpListener, sync::Mutex};
use tonic::{Request, Response, Status};
use tracing::instrument;

use super::{internal_error, invalid_argument};
use crate::{
    generated::batch_prover::{
        api_server::{Api as BatchProverApi, ApiServer as BatchApiServer},
        ProveBatchRequest, ProveBatchResponse,
    },
    utils::MIDEN_PROVING_SERVICE,
};

// BATCH RPC PROVER
// ================================================================================================

pub struct BatchProverRpcListener {
    pub api_service: BatchApiServer<BatchProverRpcApi>,
    pub listener: TcpListener,
}

impl BatchProverRpcListener {
    pub fn new(listener: TcpListener) -> Self {
        let api_service = BatchApiServer::new(BatchProverRpcApi::default());
        Self { listener, api_service }
    }
}

pub struct BatchProverRpcApi {
    local_prover: Mutex<LocalBatchProver>,
}

impl Default for BatchProverRpcApi {
    fn default() -> Self {
        Self {
            local_prover: Mutex::new(LocalBatchProver::new(MIN_PROOF_SECURITY_LEVEL)),
        }
    }
}

#[async_trait::async_trait]
impl BatchProverApi for BatchProverRpcApi {
    #[instrument(
        target = MIDEN_PROVING_SERVICE,
        name = "batch:prove_batch",
        skip_all,
        ret(level = "debug"),
        fields(batch_id = tracing::field::Empty),
        err
    )]
    async fn prove_batch(
        &self,
        request: Request<ProveBatchRequest>,
    ) -> Result<Response<ProveBatchResponse>, tonic::Status> {
        // Try to acquire a permit without waiting
        let prover = self
            .local_prover
            .try_lock()
            .map_err(|_| Status::resource_exhausted("Server is busy handling another request"))?;

        let proposed_batch = ProposedBatch::read_from_bytes(&request.get_ref().proposed_batch)
            .map_err(invalid_argument)?;

        let proof = prover.prove(proposed_batch).map_err(internal_error)?;

        // Record the batch_id in the current tracing span
        let batch_id = proof.id();
        tracing::Span::current().record("batch_id", tracing::field::display(&batch_id));

        Ok(Response::new(ProveBatchResponse { proven_batch: proof.to_bytes() }))
    }
}
