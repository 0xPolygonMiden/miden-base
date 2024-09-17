use miden_objects::transaction::TransactionWitness;
use miden_tx::{
    utils::{Deserializable, Serializable},
    LocalTransactionProver, TransactionProver,
};
use miden_tx_prover::{
    generated::api::api_server, ProveTransactionRequest, ProveTransactionResponse,
};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Request, Response};
use tracing::info;
use winter_maybe_async::maybe_await;

pub struct Rpc {
    pub(crate) api_service: api_server::ApiServer<RpcApi>,
    pub(crate) listener: TcpListener,
}
#[derive(Clone)]
struct RpcApi;

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

#[tokio::main]
async fn main() {
    let rpc = Rpc {
        listener: tokio::net::TcpListener::bind("0.0.0.0:50051").await.unwrap(),
        api_service: api_server::ApiServer::new(RpcApi),
    };

    info!("Server listening on {}", rpc.listener.local_addr().unwrap());

    // build our application with a route
    tonic::transport::Server::builder()
        .accept_http1(true)
        .add_service(tonic_web::enable(rpc.api_service))
        .serve_with_incoming(TcpListenerStream::new(rpc.listener))
        .await
        .unwrap();
}
