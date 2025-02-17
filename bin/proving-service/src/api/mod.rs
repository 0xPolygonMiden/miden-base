use miden_objects::{
    batch::ProposedBatch, transaction::TransactionWitness, MIN_PROOF_SECURITY_LEVEL,
};
use miden_tx::{
    utils::{Deserializable, Serializable},
    LocalTransactionProver, TransactionProver,
};
use miden_tx_batch_prover::LocalBatchProver;
use tokio::{net::TcpListener, sync::Mutex};
use tonic::{Request, Response, Status};
use tracing::instrument;

use crate::{
    generated::{
        api_server::{Api as ProverApi, ApiServer},
        ProveRequest, ProveResponse,
    },
    utils::MIDEN_PROVING_SERVICE,
};

pub struct RpcListener {
    pub api_service: ApiServer<ProverRpcApi>,
    pub listener: TcpListener,
}

impl RpcListener {
    pub fn new(listener: TcpListener, is_tx_prover: bool, is_batch_prover: bool) -> Self {
        let prover_rpc_api = ProverRpcApi::new(is_tx_prover, is_batch_prover);
        let api_service = ApiServer::new(prover_rpc_api);
        Self { listener, api_service }
    }
}

pub struct ProverRpcApi {
    tx_prover: Option<Mutex<LocalTransactionProver>>,
    batch_prover: Option<Mutex<LocalBatchProver>>,
}

impl ProverRpcApi {
    pub fn new(is_tx_prover: bool, is_batch_prover: bool) -> Self {
        let tx_prover = if is_tx_prover {
            Some(Mutex::new(LocalTransactionProver::default()))
        } else {
            None
        };

        let batch_prover = if is_batch_prover {
            Some(Mutex::new(LocalBatchProver::new(MIN_PROOF_SECURITY_LEVEL)))
        } else {
            None
        };

        Self { tx_prover, batch_prover }
    }
    #[instrument(
        target = MIDEN_PROVING_SERVICE,
        name = "remote_prover:prove_tx",
        skip_all,
        ret(level = "debug"),
        fields(id = tracing::field::Empty),
        err
    )]
    pub fn prove_tx(
        &self,
        request: Request<ProveRequest>,
    ) -> Result<Response<ProveResponse>, tonic::Status> {
        let tx_prover = self
            .tx_prover
            .as_ref()
            .ok_or(Status::unimplemented("Transaction prover is not enabled"))?;
        let prover = tx_prover
            .try_lock()
            .map_err(|_| Status::resource_exhausted("Server is busy handling another request"))?;

        let transaction_witness = TransactionWitness::read_from_bytes(&request.get_ref().payload)
            .map_err(invalid_argument)?;

        let proof = prover.prove(transaction_witness).map_err(internal_error)?;

        // Record the transaction_id in the current tracing span
        let transaction_id = proof.id();
        tracing::Span::current().record("id", tracing::field::display(&transaction_id));

        Ok(Response::new(ProveResponse { payload: proof.to_bytes() }))
    }

    #[instrument(
        target = MIDEN_PROVING_SERVICE,
        name = "remote_prover:prove_batch",
        skip_all,
        ret(level = "debug"),
        fields(id = tracing::field::Empty),
        err
    )]
    pub fn prove_batch(
        &self,
        request: Request<ProveRequest>,
    ) -> Result<Response<ProveResponse>, tonic::Status> {
        let batch_prover = self
            .batch_prover
            .as_ref()
            .ok_or(Status::unimplemented("Batch prover is not enabled"))?;
        let prover = batch_prover
            .try_lock()
            .map_err(|_| Status::resource_exhausted("Server is busy handling another request"))?;

        let batch =
            ProposedBatch::read_from_bytes(&request.get_ref().payload).map_err(invalid_argument)?;

        let proof = prover.prove(batch).map_err(internal_error)?;

        // Record the batch_id in the current tracing span
        let batch_id = proof.id();
        tracing::Span::current().record("id", tracing::field::display(&batch_id));

        Ok(Response::new(ProveResponse { payload: proof.to_bytes() }))
    }
}

#[async_trait::async_trait]
impl ProverApi for ProverRpcApi {
    #[instrument(
        target = MIDEN_PROVING_SERVICE,
        name = "remote_prover:prove",
        skip_all,
        ret(level = "debug"),
        fields(id = tracing::field::Empty),
        err
    )]
    async fn prove(
        &self,
        request: Request<ProveRequest>,
    ) -> Result<Response<ProveResponse>, tonic::Status> {
        match request.get_ref().proof_type {
            0 => self.prove_tx(request),
            1 => self.prove_batch(request),
            _ => Err(internal_error("Invalid proof type")),
        }
    }
}

// UTILITIES
// ================================================================================================

/// Formats an error
fn internal_error<E: core::fmt::Debug>(err: E) -> Status {
    Status::internal(format!("{:?}", err))
}

fn invalid_argument<E: core::fmt::Debug>(err: E) -> Status {
    Status::invalid_argument(format!("{:?}", err))
}
