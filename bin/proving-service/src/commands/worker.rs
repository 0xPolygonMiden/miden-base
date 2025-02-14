use clap::Parser;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic_health::server::health_reporter;
use tracing::{info, instrument};

use crate::{api::RpcListener, generated::api_server::ApiServer, utils::MIDEN_PROVING_SERVICE};

/// Defines the possible types of provers that a worker can be.
#[derive(Debug, Parser, Clone, Default)]
pub struct ProverTypeSupport {
    /// Mark the worker as a transaction prover
    #[clap(short, long, default_value = "false")]
    tx_prover: bool,
    /// Mark the worker as a batch prover
    #[clap(short, long, default_value = "false")]
    batch_prover: bool,
}

impl ProverTypeSupport {
    /// Checks if the worker is a transaction prover.
    pub fn supports_transaction(&self) -> bool {
        self.tx_prover
    }

    /// Checks if the worker is a batch prover.
    pub fn supports_batch(&self) -> bool {
        self.batch_prover
    }

    /// Mark the worker as a transaction prover.
    pub fn with_transaction(&mut self) {
        self.tx_prover = true;
    }

    /// Mark the worker as a batch prover.
    pub fn with_batch(&mut self) {
        self.batch_prover = true;
    }
}

/// Starts a worker.
#[derive(Debug, Parser)]
pub struct StartWorker {
    /// The host of the worker
    #[clap(short, long, default_value = "0.0.0.0")]
    host: String,
    /// The port of the worker
    #[clap(short, long, default_value = "50051")]
    port: u16,
    /// The type of prover that the worker will be
    #[clap(flatten)]
    prover_type: ProverTypeSupport,
}

impl StartWorker {
    /// Starts a worker.
    ///
    /// This method receives the host and port from the CLI and starts a worker on that address.
    /// In case that one of the parameters is not provided, it will default to `0.0.0.0` for the
    /// host and `50051` for the port.
    ///
    /// The worker includes a health reporter that will mark the service as serving, following the
    /// [gRPC health checking protocol](
    /// https://github.com/grpc/grpc-proto/blob/master/grpc/health/v1/health.proto).
    #[instrument(target = MIDEN_PROVING_SERVICE, name = "worker:execute")]
    pub async fn execute(&self) -> Result<(), String> {
        let worker_addr = format!("{}:{}", self.host, self.port);
        let rpc = RpcListener::new(
            TcpListener::bind(&worker_addr).await.map_err(|err| err.to_string())?,
            self.prover_type.clone(),
        );

        info!(
            "Server listening on {}",
            rpc.listener.local_addr().map_err(|err| err.to_string())?
        );

        // Create a health reporter
        let (mut health_reporter, health_service) = health_reporter();

        // Mark the service as serving
        health_reporter.set_serving::<ApiServer<RpcListener>>().await;

        tonic::transport::Server::builder()
            .accept_http1(true)
            .add_service(tonic_web::enable(rpc.api_service))
            .add_service(health_service)
            .serve_with_incoming(TcpListenerStream::new(rpc.listener))
            .await
            .map_err(|err| err.to_string())?;

        Ok(())
    }
}
