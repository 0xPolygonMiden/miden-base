use miden_block_prover::LocalBlockProver;
use miden_objects::{
    batch::ProposedBatch, block::ProposedBlock, transaction::TransactionWitness,
    utils::Serializable, MIN_PROOF_SECURITY_LEVEL,
};
use miden_tx::{LocalTransactionProver, TransactionProver};
use miden_tx_batch_prover::LocalBatchProver;
use tokio::{net::TcpListener, sync::Mutex};
use tonic::{Request, Response, Status};
use tracing::instrument;

use crate::{
    commands::worker::ProverTypeSupport,
    generated::{
        api_server::{Api as ProverApi, ApiServer},
        ProofType, ProvingRequest, ProvingResponse,
    },
    utils::MIDEN_PROVING_SERVICE,
};

pub struct RpcListener {
    pub api_service: ApiServer<ProverRpcApi>,
    pub listener: TcpListener,
}

impl RpcListener {
    pub fn new(listener: TcpListener, prover_type_support: ProverTypeSupport) -> Self {
        let prover_rpc_api = ProverRpcApi::new(prover_type_support);
        let api_service = ApiServer::new(prover_rpc_api);
        Self { listener, api_service }
    }
}
struct Provers {
    tx_prover: Option<LocalTransactionProver>,
    batch_prover: Option<LocalBatchProver>,
    block_prover: Option<LocalBlockProver>,
}

impl Provers {
    fn new(prover_type_support: ProverTypeSupport) -> Self {
        let tx_prover = if prover_type_support.supports_transaction() {
            Some(LocalTransactionProver::default())
        } else {
            None
        };

        let batch_prover = if prover_type_support.supports_batch() {
            Some(LocalBatchProver::new(MIN_PROOF_SECURITY_LEVEL))
        } else {
            None
        };

        let block_prover = if prover_type_support.supports_block() {
            Some(LocalBlockProver::new(MIN_PROOF_SECURITY_LEVEL))
        } else {
            None
        };

        Self { tx_prover, batch_prover, block_prover }
    }
}

pub struct ProverRpcApi {
    provers: Mutex<Provers>,
}

impl ProverRpcApi {
    pub fn new(prover_type_support: ProverTypeSupport) -> Self {
        let provers = Mutex::new(Provers::new(prover_type_support));

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
        transaction_witness: TransactionWitness,
    ) -> Result<Response<ProvingResponse>, tonic::Status> {
        let prover = self
            .provers
            .try_lock()
            .map_err(|_| Status::resource_exhausted("Server is busy handling another request"))?;

        let prover = prover
            .tx_prover
            .as_ref()
            .ok_or(Status::unimplemented("Transaction prover is not enabled"))?;

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
        proposed_batch: ProposedBatch,
    ) -> Result<Response<ProvingResponse>, tonic::Status> {
        let prover = self
            .provers
            .try_lock()
            .map_err(|_| Status::resource_exhausted("Server is busy handling another request"))?;

        let prover = prover
            .batch_prover
            .as_ref()
            .ok_or(Status::unimplemented("Batch prover is not enabled"))?;

        let proven_batch = prover.prove(proposed_batch).map_err(internal_error)?;

        // Record the batch_id in the current tracing span
        let batch_id = proven_batch.id();
        tracing::Span::current().record("id", tracing::field::display(&batch_id));

        Ok(Response::new(ProvingResponse { payload: proven_batch.to_bytes() }))
    }

    #[instrument(
        target = MIDEN_PROVING_SERVICE,
        name = "proving_service:prove_block",
        skip_all,
        ret(level = "debug"),
        fields(id = tracing::field::Empty),
        err
    )]
    pub fn prove_block(
        &self,
        proposed_block: ProposedBlock,
    ) -> Result<Response<ProvingResponse>, tonic::Status> {
        let prover = self
            .provers
            .try_lock()
            .map_err(|_| Status::resource_exhausted("Server is busy handling another request"))?;

        let prover = prover
            .block_prover
            .as_ref()
            .ok_or(Status::unimplemented("Block prover is not enabled"))?;

        let proven_block = prover.prove(proposed_block).map_err(internal_error)?;

        // Record the commitment of the block in the current tracing span
        let block_id = proven_block.hash();

        tracing::Span::current().record("id", tracing::field::display(&block_id));

        Ok(Response::new(ProvingResponse { payload: proven_block.to_bytes() }))
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
        match request.get_ref().proof_type() {
            ProofType::Transaction => {
                let tx_witness = request.into_inner().try_into().map_err(invalid_argument)?;
                self.prove_tx(tx_witness)
            },
            ProofType::Batch => {
                let proposed_batch = request.into_inner().try_into().map_err(invalid_argument)?;
                self.prove_batch(proposed_batch)
            },
            ProofType::Block => {
                let proposed_block = request.into_inner().try_into().map_err(invalid_argument)?;
                self.prove_block(proposed_block)
            },
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
