use std::{fs::File, io::Write};

use clap::Parser;
use figment::{
    providers::{Format, Toml},
    Figment,
};
use init::Init;
use miden_tx_prover::PROVER_SERVICE_CONFIG_FILE_NAME;
use proxy::StartProxy;
use serde::{Deserialize, Serialize};
use update_workers::UpdateWorkers;
use worker::StartWorker;

pub mod init;
pub mod proxy;
pub mod update_workers;
pub mod worker;

/// Configuration of the proxy.
///
/// It is stored in a TOML file, which will be created by the `init` command.
/// It allows manual modification of the configuration file.
#[derive(Serialize, Deserialize)]
pub struct ProxyConfig {
    /// List of workers used by the proxy.
    pub workers: Vec<WorkerConfig>,
    /// Host of the proxy.
    pub host: String,
    /// Port of the proxy.
    pub port: u16,
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
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            workers: vec![WorkerConfig::new("0.0.0.0", 8083), WorkerConfig::new("0.0.0.0", 8084)],
            host: "0.0.0.0".into(),
            port: 8082,
            timeout_secs: 100,
            connection_timeout_secs: 10,
            max_queue_items: 10,
            max_retries_per_request: 1,
            max_req_per_sec: 5,
            available_workers_polling_time_ms: 20,
        }
    }
}

impl ProxyConfig {
    /// Loads config file from current directory and default filename and returns it
    ///
    /// This function will look for the configuration file with the name defined at the
    /// [PROVER_SERVICE_CONFIG_FILE_NAME] constant in the current directory.
    pub(crate) fn load_config_from_file() -> Result<ProxyConfig, String> {
        let mut current_dir = std::env::current_dir().map_err(|err| err.to_string())?;
        current_dir.push(PROVER_SERVICE_CONFIG_FILE_NAME);
        let config_path = current_dir.as_path();

        Figment::from(Toml::file(config_path))
            .extract()
            .map_err(|err| format!("Failed to load {} config file: {err}", config_path.display()))
    }

    pub(crate) fn save_to_config_file(&self) -> Result<(), String> {
        let mut current_dir = std::env::current_dir().map_err(|err| err.to_string())?;
        current_dir.push(PROVER_SERVICE_CONFIG_FILE_NAME);
        let config_path = current_dir.as_path();

        let config_as_toml_string = toml::to_string_pretty(self)
            .map_err(|err| format!("error formatting config: {err}"))?;

        let mut file_handle = File::options()
            .write(true)
            .truncate(true)
            .open(config_path)
            .map_err(|err| format!("error opening the file: {err}"))?;

        file_handle
            .write(config_as_toml_string.as_bytes())
            .map_err(|err| format!("error writing to file: {err}"))?;

        println!("Config updated successfully");

        Ok(())
    }
}

/// Configuration for a worker
#[derive(Serialize, Deserialize)]
pub struct WorkerConfig {
    pub host: String,
    pub port: u16,
}

impl WorkerConfig {
    pub fn new(host: &str, port: u16) -> Self {
        Self { host: host.into(), port }
    }
}

/// Root CLI struct
#[derive(Parser, Debug)]
#[clap(
    name = "miden-tx-prover",
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
    /// [miden_tx_prover::PROVER_SERVICE_CONFIG_FILE_NAME] constant.
    Init(Init),
    /// Starts the workers defined in the config file.
    StartWorker(StartWorker),
    /// Starts the proxy defined in the config file.
    StartProxy(StartProxy),
    /// Updates the workers defined in the config file.
    ///
    /// This method will make a request to the proxy defined in the config file to update the
    /// workers. It will update the configuration file with the new list of workers.
    UpdateWorkers(UpdateWorkers),
}

/// CLI entry point
impl Cli {
    pub fn execute(&self) -> Result<(), String> {
        match &self.action {
            // For the `StartWorker` command, we need to create a new runtime and run the worker
            Command::StartWorker(worker_init) => {
                let rt = tokio::runtime::Runtime::new()
                    .map_err(|e| format!("Failed to create runtime: {:?}", e))?;
                rt.block_on(worker_init.execute())
            },
            Command::StartProxy(proxy_init) => proxy_init.execute(),
            Command::Init(init) => {
                // Init does not require async, so run directly
                init.execute()
            },
            Command::UpdateWorkers(update_workers) => update_workers.execute(),
        }
    }
}
