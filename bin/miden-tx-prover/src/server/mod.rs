use generated::api::api_server;
use miden_objects::transaction::TransactionWitness;
use miden_tx::{
    utils::{Deserializable, Serializable},
    LocalTransactionProver, TransactionProver,
};
use tokio::net::TcpListener;
use tonic::{Request, Response};
use tracing::info;
use winter_maybe_async::maybe_await;

use crate::{ProveTransactionRequest, ProveTransactionResponse};

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

#[tonic::async_trait]
impl api_server::Api for RpcApi {
    async fn prove_transaction(
        &self,
        request: Request<ProveTransactionRequest>,
    ) -> Result<Response<ProveTransactionResponse>, tonic::Status> {
        info!("Received request to prove transaction");
        let prover = LocalTransactionProver::default();

        let transaction_witness =
            TransactionWitness::read_from_bytes(&request.get_ref().transaction_witness).unwrap();

        let proof = maybe_await!(prover.prove(transaction_witness)).unwrap();

        Ok(Response::new(ProveTransactionResponse { proven_transaction: proof.to_bytes() }))
    }
}
