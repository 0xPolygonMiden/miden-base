use clap::Parser;
use proxy::StartProxy;
use tracing::instrument;
use update_workers::{AddWorkers, RemoveWorkers, UpdateWorkers};
use worker::StartWorker;

use crate::utils::MIDEN_PROVING_SERVICE;

pub mod proxy;
pub mod update_workers;
pub mod worker;

pub(crate) const PROXY_HOST: &str = "0.0.0.0";

#[derive(Debug, Parser)]
pub(crate) struct ProxyConfig {
    /// Interval in milliseconds at which the system polls for available workers to assign new
    /// tasks.
    #[clap(long, default_value = "20", env = "MPS_AVAILABLE_WORKERS_POLLING_INTERVAL_MS")]
    pub(crate) available_workers_polling_interval_ms: u64,
    /// Maximum time in seconds to establish a connection.
    #[clap(long, default_value = "10", env = "MPS_CONNECTION_TIMEOUT_SECS")]
    pub(crate) connection_timeout_secs: u64,
    /// Health check interval in seconds.
    #[clap(long, default_value = "10", env = "MPS_HEALTH_CHECK_INTERVAL_SECS")]
    pub(crate) health_check_interval_secs: u64,
    /// Maximum number of items in the queue.
    #[clap(long, default_value = "10", env = "MPS_MAX_QUEUE_ITEMS")]
    pub(crate) max_queue_items: usize,
    /// Maximum number of requests per second per IP address.
    #[clap(long, default_value = "5", env = "MPS_MAX_REQ_PER_SEC")]
    pub(crate) max_req_per_sec: isize,
    /// Maximum number of retries per request.
    #[clap(long, default_value = "1", env = "MPS_MAX_RETRIES_PER_REQUEST")]
    pub(crate) max_retries_per_request: usize,
    /// Metrics configurations.
    #[clap(flatten)]
    pub(crate) metrics_config: MetricsConfig,
    /// Port of the proxy.
    #[clap(long, default_value = "8082", env = "MPS_PORT")]
    pub(crate) port: u16,
    /// Maximum time in seconds allowed for a request to complete. Once exceeded, the request is
    /// aborted.
    #[clap(long, default_value = "100", env = "MPS_TIMEOUT_SECS")]
    pub(crate) timeout_secs: u64,
    /// Control port.
    #[clap(long, default_value = "8083", env = "MPS_CONTROL_PORT")]
    pub(crate) control_port: u16,
    /// Supported proof types.
    #[clap(
        long,
        default_value = "transaction,batch,block",
        env = "MPS_SUPPORTED_PROOF_TYPES"
    )]
    pub(crate) supported_proof_types: String,
}

#[derive(Debug, Clone, clap::Parser)]
pub struct MetricsConfig {
    /// Port for Prometheus-compatible metrics
    /// If specified, metrics will be enabled on this port. If not specified, metrics will be
    /// disabled.
    #[arg(long, env = "MPS_METRICS_PORT")]
    pub metrics_port: Option<u16>,
}

/// Root CLI struct
#[derive(Parser, Debug)]
#[clap(
    name = "miden-proving-service",
    about = "A stand-alone service for proving Miden transactions.",
    version,
    rename_all = "kebab-case"
)]
pub struct Cli {
    #[clap(subcommand)]
    action: Command,
}

/// CLI actions
#[derive(Debug, Parser)]
pub enum Command {
    /// Starts the workers with the configuration defined in the command.
    StartWorker(StartWorker),
    /// Starts the proxy.
    StartProxy(StartProxy),
    /// Adds workers to the proxy.
    ///
    /// This command will make a request to the proxy to add the specified workers.
    AddWorkers(AddWorkers),
    /// Removes workers from the proxy.
    ///
    /// This command will make a request to the proxy to remove the specified workers.
    RemoveWorkers(RemoveWorkers),
}

/// CLI entry point
impl Cli {
    #[instrument(target = MIDEN_PROVING_SERVICE, name = "cli:execute", skip_all, ret(level = "info"), err)]
    pub async fn execute(&self) -> Result<(), String> {
        match &self.action {
            // For the `StartWorker` command, we need to create a new runtime and run the worker
            Command::StartWorker(worker_init) => worker_init.execute().await,
            Command::StartProxy(proxy_init) => proxy_init.execute().await,
            Command::AddWorkers(update_workers) => {
                let update_workers: UpdateWorkers = update_workers.clone().into();
                update_workers.execute().await
            },
            Command::RemoveWorkers(update_workers) => {
                let update_workers: UpdateWorkers = update_workers.clone().into();
                update_workers.execute().await
            },
        }
    }
}
