use clap::Parser;
use figment::{
    providers::{Format, Toml},
    Figment,
};
use init::Init;
use proxy::StartProxy;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use update_workers::{AddWorkers, RemoveWorkers, UpdateWorkers};
use worker::StartWorker;

use crate::utils::{MIDEN_PROVING_SERVICE, PROVING_SERVICE_CONFIG_FILE_NAME};

pub mod init;
pub mod proxy;
pub mod update_workers;
pub mod worker;

/// Configuration of the proxy.
///
/// It is stored in a TOML file, which will be created by the `init` command.
/// It allows manual modification of the configuration file.
#[derive(Debug, Serialize, Deserialize)]
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
    /// Loads config file from current directory and default filename and returns it
    ///
    /// This function will look for the configuration file with the name defined at the
    /// [PROVING_SERVICE_CONFIG_FILE_NAME] constant in the current directory.
    pub(crate) fn load_config_from_file() -> Result<ProxyConfig, String> {
        let mut current_dir = std::env::current_dir().map_err(|err| err.to_string())?;
        current_dir.push(PROVING_SERVICE_CONFIG_FILE_NAME);
        let config_path = current_dir.as_path();

        Figment::from(Toml::file(config_path))
            .extract()
            .map_err(|err| format!("Failed to load {} config file: {err}", config_path.display()))
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
    /// Creates a config file for the proxy.
    ///
    /// This method will create a new config file in the current working directory with default
    /// values. The file will be named as defined in the
    /// [PROVING_SERVICE_CONFIG_FILE_NAME] constant.
    Init(Init),
    /// Starts the workers with the configuration defined in the command.
    StartWorker(StartWorker),
    /// Starts the proxy defined in the config file.
    StartProxy(StartProxy),
    /// Adds workers to the proxy.
    ///
    /// This method will make a request to the proxy defined in the config file to add workers.
    AddWorkers(AddWorkers),
    /// Removes workers from the proxy.
    ///
    /// This method will make a request to the proxy defined in the config file to remove workers.
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
            Command::Init(init) => {
                // Init does not require async, so run directly
                init.execute()
            },
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
