use clap::Parser;
use proxy::StartProxy;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use update_workers::{AddWorkers, RemoveWorkers, UpdateWorkers};
use worker::StartWorker;

use crate::{error::ProvingServiceError, utils::MIDEN_PROVING_SERVICE};

pub mod proxy;
pub mod update_workers;
pub mod worker;

/// Prefix for environment variables.
const ENV_PREFIX: &str = "MPS_";

/// Configuration of the proxy.
///
/// It is stored in a TOML file, which will be created by the `init` command.
/// It allows manual modification of the configuration file.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProxyConfig {
    /// Host of the proxy.
    pub host: String,
    /// Port of the proxy.
    pub port: u16,
    /// Worker update service port.
    pub workers_update_port: u16,
    /// Maximum time in seconds to complete the entire request.
    pub timeout_secs: u64,
    /// Maximum time in seconds to establish a connection.
    pub connection_timeout_secs: u64,
    /// Maximum number of items in the queue.
    pub max_queue_items: usize,
    /// Maximum number of retries per request.
    pub max_retries_per_request: usize,
    /// Maximum number of requests per second per IP address.
    pub max_req_per_sec: isize,
    /// Time in milliseconds to poll available workers.
    pub available_workers_polling_time_ms: u64,
    /// Health check interval in seconds.
    pub health_check_interval_secs: u64,
    /// Prometheus metrics host.
    pub prometheus_host: String,
    /// Prometheus metrics port.
    pub prometheus_port: u16,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 8082,
            timeout_secs: 100,
            connection_timeout_secs: 10,
            max_queue_items: 10,
            max_retries_per_request: 1,
            max_req_per_sec: 5,
            available_workers_polling_time_ms: 20,
            health_check_interval_secs: 1,
            prometheus_host: "127.0.0.1".into(),
            prometheus_port: 6192,
            workers_update_port: 8083,
        }
    }
}

impl ProxyConfig {
    /// Load config from environment variables using defaults in case of missing values.
    ///
    /// The environment variables must be prefixed with [`ENV_PREFIX`].
    pub(crate) fn load() -> Result<ProxyConfig, ProvingServiceError> {
        let mut config = ProxyConfig::default();

        config.host = std::env::var(format!("{}HOST", ENV_PREFIX)).unwrap_or(config.host);
        config.port = std::env::var(format!("{}PORT", ENV_PREFIX))
            .unwrap_or(config.port.to_string())
            .parse()?;
        config.timeout_secs = std::env::var(format!("{}TIMEOUT_SECS", ENV_PREFIX))
            .unwrap_or(config.timeout_secs.to_string())
            .parse()?;
        config.connection_timeout_secs =
            std::env::var(format!("{}CONNECTION_TIMEOUT_SECS", ENV_PREFIX))
                .unwrap_or(config.connection_timeout_secs.to_string())
                .parse()?;
        config.max_queue_items = std::env::var(format!("{}MAX_QUEUE_ITEMS", ENV_PREFIX))
            .unwrap_or(config.max_queue_items.to_string())
            .parse()?;
        config.max_retries_per_request =
            std::env::var(format!("{}MAX_RETRIES_PER_REQUEST", ENV_PREFIX))
                .unwrap_or(config.max_retries_per_request.to_string())
                .parse()?;
        config.max_req_per_sec = std::env::var(format!("{}MAX_REQ_PER_SEC", ENV_PREFIX))
            .unwrap_or(config.max_req_per_sec.to_string())
            .parse()?;
        config.available_workers_polling_time_ms =
            std::env::var(format!("{}AVAILABLE_WORKERS_POLLING_TIME_MS", ENV_PREFIX))
                .unwrap_or(config.available_workers_polling_time_ms.to_string())
                .parse()?;
        config.health_check_interval_secs =
            std::env::var(format!("{}HEALTH_CHECK_INTERVAL_SECS", ENV_PREFIX))
                .unwrap_or(config.health_check_interval_secs.to_string())
                .parse()?;
        config.prometheus_host = std::env::var(format!("{}PROMETHEUS_HOST", ENV_PREFIX))
            .unwrap_or(config.prometheus_host);
        config.prometheus_port = std::env::var(format!("{}PROMETHEUS_PORT", ENV_PREFIX))
            .unwrap_or(config.prometheus_port.to_string())
            .parse()?;
        config.workers_update_port = std::env::var(format!("{}WORKERS_UPDATE_PORT", ENV_PREFIX))
            .unwrap_or(config.workers_update_port.to_string())
            .parse()?;

        Ok(config)
    }
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
    /// This method will make a request to the proxy adding workers.
    AddWorkers(AddWorkers),
    /// Removes workers from the proxy.
    ///
    /// This method will make a request to the proxy removing workers.
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
