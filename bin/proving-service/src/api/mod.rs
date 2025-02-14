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
        ProvingRequest, ProvingResponse,
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
struct Provers {
    tx_prover: Option<LocalTransactionProver>,
    batch_prover: Option<LocalBatchProver>,
}

impl Provers {
    fn new(is_tx_prover: bool, is_batch_prover: bool) -> Self {
        let tx_prover = if is_tx_prover {
            Some(LocalTransactionProver::default())
        } else {
            None
        };

        let batch_prover = if is_batch_prover {
            Some(LocalBatchProver::new(MIN_PROOF_SECURITY_LEVEL))
        } else {
            None
        };

        Self { tx_prover, batch_prover }
    }
}

pub struct ProverRpcApi {
    provers: Mutex<Provers>,
}

impl ProverRpcApi {
    pub fn new(is_tx_prover: bool, is_batch_prover: bool) -> Self {
        let provers = Mutex::new(Provers::new(is_tx_prover, is_batch_prover));

        Self { provers }
    }
    #[instrument(
        target = MIDEN_PROVING_SERVICE,
        name = "proving_service:prove_tx",
        skip_all,
        ret(level = "debug"),
        fields(id = tracing::field::Empty),
        err
    )]
    pub fn prove_tx(
        &self,
        request: Request<ProvingRequest>,
    ) -> Result<Response<ProvingResponse>, tonic::Status> {
        let prover = self
            .provers
            .try_lock()
            .map_err(|_| Status::resource_exhausted("Server is busy handling another request"))?;

        let prover = prover
            .tx_prover
            .as_ref()
            .ok_or(Status::unimplemented("Transaction prover is not enabled"))?;

        let transaction_witness = TransactionWitness::read_from_bytes(&request.get_ref().payload)
            .map_err(invalid_argument)?;

        let proof = prover.prove(transaction_witness).map_err(internal_error)?;

        // Record the transaction_id in the current tracing span
        let transaction_id = proof.id();
        tracing::Span::current().record("id", tracing::field::display(&transaction_id));

        Ok(Response::new(ProvingResponse { payload: proof.to_bytes() }))
    }

    #[instrument(
        target = MIDEN_PROVING_SERVICE,
        name = "proving_service:prove_batch",
        skip_all,
        ret(level = "debug"),
        fields(id = tracing::field::Empty),
        err
    )]
    pub fn prove_batch(
        &self,
        request: Request<ProvingRequest>,
    ) -> Result<Response<ProvingResponse>, tonic::Status> {
        let prover = self
            .provers
            .try_lock()
            .map_err(|_| Status::resource_exhausted("Server is busy handling another request"))?;

        let prover = prover
            .batch_prover
            .as_ref()
            .ok_or(Status::unimplemented("Batch prover is not enabled"))?;

        let batch =
            ProposedBatch::read_from_bytes(&request.get_ref().payload).map_err(invalid_argument)?;

        let proof = prover.prove(batch).map_err(internal_error)?;

        // Record the batch_id in the current tracing span
        let batch_id = proof.id();
        tracing::Span::current().record("id", tracing::field::display(&batch_id));

        Ok(Response::new(ProvingResponse { payload: proof.to_bytes() }))
    }
}

#[async_trait::async_trait]
impl ProverApi for ProverRpcApi {
    #[instrument(
        target = MIDEN_PROVING_SERVICE,
        name = "proving_service:prove",
        skip_all,
        ret(level = "debug"),
        fields(id = tracing::field::Empty),
        err
    )]
    async fn prove(
        &self,
        request: Request<ProvingRequest>,
    ) -> Result<Response<ProvingResponse>, tonic::Status> {
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
