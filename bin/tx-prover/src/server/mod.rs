use std::sync::Arc;

use generated::api::{api_server, ProveTransactionRequest, ProveTransactionResponse};
use miden_objects::transaction::{ProvenTransaction, TransactionWitness};
use miden_tx::{
    utils::{Deserializable, Serializable},
    LocalTransactionProver, TransactionProver, TransactionProverError,
};
use tokio::{net::TcpListener, sync::Semaphore};
use tonic::{Request, Response, Status};
use tracing::info;
use winter_maybe_async::maybe_await;

pub mod generated;

pub struct Rpc {
    pub api_service: api_server::ApiServer<RpcApi>,
    pub listener: TcpListener,
}

impl Rpc {
    pub fn new(listener: TcpListener) -> Self {
        let semaphore = Arc::new(Semaphore::new(1));
        let api_service = api_server::ApiServer::new(RpcApi::new(semaphore.clone()));
        Self { listener, api_service }
    }
}

pub struct RpcApi {
    prover: LocalTransactionProver,
    semaphore: Arc<Semaphore>,
}

impl RpcApi {
    pub fn new(semaphore: Arc<Semaphore>) -> Self {
        Self {
            prover: LocalTransactionProver::default(),
            semaphore,
        }
    }
}

unsafe impl Send for RpcApi {}
unsafe impl Sync for RpcApi {}

#[tonic::async_trait]
impl api_server::Api for RpcApi {
    async fn prove_transaction(
        &self,
        request: Request<ProveTransactionRequest>,
    ) -> Result<Response<ProveTransactionResponse>, tonic::Status> {
        info!("Received request to prove transaction");

        // Try to acquire a permit without waiting
        let permit = self.semaphore.try_acquire();
        let _permit = match permit {
            Ok(permit) => permit,
            Err(_) => {
                return Err(Status::resource_exhausted("Server is busy handling another request"));
            },
        };

        let transaction_witness =
            TransactionWitness::read_from_bytes(&request.get_ref().transaction_witness)
                .map_err(invalid_argument)?;

        let proof = maybe_await!(self.prover.prove(transaction_witness)).map_err(internal_error)?;

        Ok(Response::new(ProveTransactionResponse { proven_transaction: proof.to_bytes() }))
    }
}

// CONVERSIONS
// ================================================================================================

impl From<ProvenTransaction> for ProveTransactionResponse {
    fn from(value: ProvenTransaction) -> Self {
        ProveTransactionResponse { proven_transaction: value.to_bytes() }
    }
}

impl TryFrom<ProveTransactionResponse> for ProvenTransaction {
    type Error = TransactionProverError;

    fn try_from(response: ProveTransactionResponse) -> Result<Self, Self::Error> {
        ProvenTransaction::read_from_bytes(&response.proven_transaction)
            .map_err(|_err| TransactionProverError::DeserializationError)
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
