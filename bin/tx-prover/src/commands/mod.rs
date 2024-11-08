use clap::Parser;
use init::Init;
use proxy::StartProxy;
use serde::{Deserialize, Serialize};
use worker::StartWorker;

pub mod init;
pub mod proxy;
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
        }
    }
}

/// Configuration for a worker
#[derive(Serialize, Deserialize)]
pub struct WorkerConfig {
    pub host: String,
    pub port: u16,
}

impl WorkerConfig {
    fn new(host: &str, port: u16) -> Self {
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
        }
    }
}
