use clap::Parser;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tracing::info;

use crate::api::RpcListener;

/// Starts the workers defined in the config file.
#[derive(Debug, Parser)]
pub struct StartWorker {
    /// The host of the worker
    #[clap(short, long, default_value = "0.0.0.0")]
    host: String,
    /// The port of the worker
    #[clap(short, long, default_value = "50051")]
    port: u16,
}

impl StartWorker {
    /// Starts the workers defined in the config file.
    ///
    /// This method will first read the config file to get the list of workers to start. It will
    /// then start a server for each worker and wait for all servers to complete.
    pub async fn execute(&self) -> Result<(), String> {
        tracing_subscriber::fmt::init();
        let worker_addr = format!("{}:{}", self.host, self.port);
        let rpc =
            RpcListener::new(TcpListener::bind(&worker_addr).await.map_err(|err| err.to_string())?);

        info!(
            "Server listening on {}",
            rpc.listener.local_addr().map_err(|err| err.to_string())?
        );

        tonic::transport::Server::builder()
            .accept_http1(true)
            .add_service(tonic_web::enable(rpc.api_service))
            .serve_with_incoming(TcpListenerStream::new(rpc.listener))
            .await
            .map_err(|err| err.to_string())?;

        Ok(())
    }
}
