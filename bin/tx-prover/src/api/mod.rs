use miden_objects::transaction::TransactionWitness;
use miden_tx::{
    utils::{Deserializable, Serializable},
    LocalTransactionProver, TransactionProver,
};
use tokio::{net::TcpListener, sync::Mutex};
use tonic::{Request, Response, Status};
use tracing::info;

use crate::generated::{
    api_server::{Api as ProverApi, ApiServer},
    ProveTransactionRequest, ProveTransactionResponse,
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

// We need to implement Send and Sync for the generated code to be able to use the prover in the
// shared context.
unsafe impl Send for ProverRpcApi {}
unsafe impl Sync for ProverRpcApi {}

#[async_trait::async_trait]
impl ProverApi for ProverRpcApi {
    async fn prove_transaction(
        &self,
        request: Request<ProveTransactionRequest>,
    ) -> Result<Response<ProveTransactionResponse>, tonic::Status> {
        info!("Received request to prove transaction");

        // Try to acquire a permit without waiting
        let prover = self
            .local_prover
            .try_lock()
            .map_err(|_| Status::resource_exhausted("Server is busy handling another request"))?;

        let transaction_witness =
            TransactionWitness::read_from_bytes(&request.get_ref().transaction_witness)
                .map_err(invalid_argument)?;

        let proof = prover.prove(transaction_witness).map_err(internal_error)?;

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
