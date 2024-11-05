use clap::Parser;
use tokio::{net::TcpListener, task};
use tokio_stream::wrappers::TcpListenerStream;
use tracing::info;

use crate::{api::RpcListener, utils::load_config_from_file};

/// Starts the workers defined in the config file.
#[derive(Debug, Parser)]
pub struct StartWorker;

impl StartWorker {
    /// Starts the workers defined in the config file.
    ///
    /// This method will first read the config file to get the list of workers to start. It will
    /// then start a server for each worker and wait for all servers to complete.
    pub async fn execute(&self) -> Result<(), String> {
        tracing_subscriber::fmt::init();
        let cli_config = load_config_from_file()?;
        let workers_addrs = cli_config
            .workers
            .iter()
            .map(|worker| format!("{}:{}", worker.host, worker.port))
            .collect::<Vec<String>>();

        let mut handles = Vec::new();

        for worker_addr in workers_addrs {
            let handle = task::spawn(async move {
                let rpc = RpcListener::new(
                    TcpListener::bind(&worker_addr).await.map_err(|err| err.to_string())?,
                );

                info!(
                    "Server listening on {}",
                    rpc.listener.local_addr().map_err(|err| err.to_string())?
                );

                tonic::transport::Server::builder()
                    .accept_http1(true)
                    .add_service(tonic_web::enable(rpc.api_service))
                    .serve_with_incoming(TcpListenerStream::new(rpc.listener))
                    .await
                    .map_err(|err| err.to_string())
            });

            handles.push(handle);
        }

        // Wait for all server tasks to complete
        for handle in handles {
            handle.await.map_err(|err| err.to_string())??;
        }

        Ok(())
    }
}
