use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic_health::server::health_reporter;
use tracing::{info, instrument};

use crate::{
    api::tx_prover_api::TxProverRpcListener, generated::tx_prover::api_server::ApiServer,
    utils::MIDEN_PROVING_SERVICE,
};

#[instrument(target = MIDEN_PROVING_SERVICE, name = "worker:start")]
pub async fn start(worker_addr: String) -> Result<(), String> {
    let rpc = TxProverRpcListener::new(
        TcpListener::bind(&worker_addr).await.map_err(|err| err.to_string())?,
    );

    info!("Server listening on {}", &worker_addr,);

    // Create a health reporter
    let (mut health_reporter, health_service) = health_reporter();

    // Mark the service as serving
    health_reporter.set_serving::<ApiServer<TxProverRpcListener>>().await;

    let service = tonic_web::enable(rpc.api_service);

    tonic::transport::Server::builder()
        .accept_http1(true)
        .add_service(service)
        .add_service(health_service)
        .serve_with_incoming(TcpListenerStream::new(rpc.listener))
        .await
        .map_err(|err| err.to_string())?;

    Ok(())
}
