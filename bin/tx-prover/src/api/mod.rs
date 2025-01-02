use miden_objects::transaction::TransactionWitness;
use miden_tx::{
    utils::{Deserializable, Serializable},
    LocalTransactionProver, TransactionProver,
};
use tokio::{net::TcpListener, sync::Mutex};
use tonic::{Request, Response, Status};
use tracing::instrument;

use crate::{
    generated::{
        api_server::{Api as ProverApi, ApiServer},
        ProveTransactionRequest, ProveTransactionResponse,
    },
    utils::MIDEN_TX_PROVER,
};

pub struct RpcListener {
    pub api_service: ApiServer<ProverRpcApi>,
    pub listener: TcpListener,
}

impl RpcListener {
    pub fn new(listener: TcpListener) -> Self {
        let api_service = ApiServer::new(ProverRpcApi::default());
        Self { listener, api_service }
    }
}

#[derive(Default)]
pub struct ProverRpcApi {
    local_prover: Mutex<LocalTransactionProver>,
}

#[async_trait::async_trait]
impl ProverApi for ProverRpcApi {
    #[instrument(
        target = MIDEN_TX_PROVER,
        name = "prover:prove_transaction",
        skip_all,
        ret(level = "debug"),
        fields(transaction_id = tracing::field::Empty),
        err
    )]
    async fn prove_transaction(
        &self,
        request: Request<ProveTransactionRequest>,
    ) -> Result<Response<ProveTransactionResponse>, tonic::Status> {
        // Try to acquire a permit without waiting
        let prover = self
            .local_prover
            .try_lock()
            .map_err(|_| Status::resource_exhausted("Server is busy handling another request"))?;

        let transaction_witness =
            TransactionWitness::read_from_bytes(&request.get_ref().transaction_witness)
                .map_err(invalid_argument)?;

        let proof = prover.prove(transaction_witness).map_err(internal_error)?;

        // Record the transaction_id in the current tracing span
        let transaction_id = proof.id();
        tracing::Span::current().record("transaction_id", tracing::field::display(&transaction_id));

        Ok(Response::new(ProveTransactionResponse { proven_transaction: proof.to_bytes() }))
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
