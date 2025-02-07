use clap::Parser;
use tracing::instrument;

use crate::{
    utils::MIDEN_PROVING_SERVICE,
    worker::{batch_prover_worker, tx_prover_worker},
};

/// Prover type.
#[derive(Debug, clap::ValueEnum, Clone)]
enum ProverType {
    Tx,
    Batch,
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
    /// Prover type
    ///
    /// The prover type can be either `tx` or `batch`.
    #[clap(short, long, default_value = "tx")]
    prover_type: ProverType,
}

impl StartWorker {
    /// Starts a worker.
    ///
    /// It uses the prover_type argument to determine which worker to start.
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

        match self.prover_type {
            ProverType::Tx => {
                tx_prover_worker::start(worker_addr).await?;
            },
            ProverType::Batch => {
                batch_prover_worker::start(worker_addr).await?;
            },
        }

        Ok(())
    }
}
