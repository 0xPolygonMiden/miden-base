use generated::api::{api_server, ProveTransactionRequest, ProveTransactionResponse};
use miden_objects::transaction::{ProvenTransaction, TransactionWitness};
use miden_tx::{
    utils::{Deserializable, Serializable},
    LocalTransactionProver, TransactionProver, TransactionProverError,
};
use tokio::net::TcpListener;
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
        let api_service = api_server::ApiServer::new(RpcApi);
        Self { listener, api_service }
    }
}

#[derive(Clone)]
pub struct RpcApi;

#[cfg(not(feature = "async"))]
#[tonic::async_trait]
impl api_server::Api for RpcApi {
    async fn prove_transaction(
        &self,
        request: Request<ProveTransactionRequest>,
    ) -> Result<Response<ProveTransactionResponse>, tonic::Status> {
        info!("Received request to prove transaction");
        let prover = LocalTransactionProver::default();

        let transaction_witness =
            TransactionWitness::read_from_bytes(&request.get_ref().transaction_witness)
                .map_err(invalid_argument)?;

        let proof = maybe_await!(prover.prove(transaction_witness)).map_err(internal_error)?;

        Ok(Response::new(ProveTransactionResponse { proven_transaction: proof.to_bytes() }))
    }
}

#[cfg(feature = "async")]
#[tonic::async_trait]
impl api_server::Api for RpcApi {
    async fn prove_transaction(
        &self,
        _request: Request<ProveTransactionRequest>,
    ) -> Result<Response<ProveTransactionResponse>, tonic::Status> {
        unimplemented!()
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
