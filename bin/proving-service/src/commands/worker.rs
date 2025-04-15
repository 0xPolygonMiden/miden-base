use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic_health::server::health_reporter;
use tracing::{info, instrument};

use crate::{api::RpcListener, generated::api_server::ApiServer, utils::MIDEN_PROVING_SERVICE};

/// Specifies the type of proving task a worker can handle.
#[derive(Debug, Clone, Copy, Default, ValueEnum, PartialEq, Serialize, Deserialize)]
pub enum ProverType {
    /// Transaction proving
    #[default]
    Transaction,
    /// Batch proving
    Batch,
    /// Block proving
    Block,
}

impl ProverType {
    /// Returns the corresponding ProofType from the generated code
    pub fn to_proof_type(&self) -> crate::generated::ProofType {
        match self {
            ProverType::Transaction => crate::generated::ProofType::Transaction,
            ProverType::Batch => crate::generated::ProofType::Batch,
            ProverType::Block => crate::generated::ProofType::Block,
        }
    }
}

impl std::fmt::Display for ProverType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProverType::Transaction => write!(f, "transaction"),
            ProverType::Batch => write!(f, "batch"),
            ProverType::Block => write!(f, "block"),
        }
    }
}

impl std::str::FromStr for ProverType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "transaction" => Ok(ProverType::Transaction),
            "batch" => Ok(ProverType::Batch),
            "block" => Ok(ProverType::Block),
            _ => Err(format!("Invalid proof type: {}", s)),
        }
    }
}

/// Starts a worker.
#[derive(Debug, Parser)]
pub struct StartWorker {
    /// The host of the worker
    #[clap(long, default_value = "0.0.0.0")]
    host: String,
    /// The port of the worker
    #[clap(long, default_value = "50051")]
    port: u16,
    /// The port of the HTTP server for the information endpoint
    #[clap(short, long, default_value = "3000")]
    info_port: u16,
    /// The type of prover that the worker will be
    #[clap(long)]
    prover_type: ProverType,
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
            self.prover_type,
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
            .add_service(tonic_web::enable(rpc.status_service))
            .add_service(health_service)
            .serve_with_incoming(TcpListenerStream::new(rpc.listener))
            .await
            .map_err(|err| err.to_string())?;

        Ok(())
    }
}
