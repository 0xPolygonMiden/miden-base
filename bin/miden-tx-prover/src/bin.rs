use std::env;

use miden_tx_prover::server::Rpc;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tracing::info;

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber with default settings for console output
    tracing_subscriber::fmt::init();

    let host = env::var("PROVER_SERVICE_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PROVER_SERVICE_PORT").unwrap_or_else(|_| "50051".to_string());
    let addr = format!("{}:{}", host, port);

    let rpc = Rpc::new(TcpListener::bind(addr).await.unwrap());

    info!("Server listening on {}", rpc.listener.local_addr().unwrap());

    // build our application with a route
    tonic::transport::Server::builder()
        .accept_http1(true)
        .add_service(tonic_web::enable(rpc.api_service))
        .serve_with_incoming(TcpListenerStream::new(rpc.listener))
        .await
        .unwrap();
}
