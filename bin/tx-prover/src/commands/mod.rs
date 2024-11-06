use clap::Parser;
use init::Init;
use proxy::StartProxy;
use serde::{Deserialize, Serialize};
use worker::StartWorker;

pub mod init;
pub mod proxy;
pub mod worker;

/// Configuration of the CLI
///
/// It is stored in a TOML file, which will be created by the `init` command.
/// It allows manual modification of the configuration file.
#[derive(Serialize, Deserialize)]
pub struct CliConfig {
    /// List of workers to start
    pub workers: Vec<WorkerConfig>,
    /// Proxy configuration
    pub proxy: ProxyConfig,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            workers: vec![WorkerConfig::default(), WorkerConfig::default()],
            proxy: ProxyConfig::default(),
        }
    }
}

/// Configuration for a worker
#[derive(Serialize, Deserialize)]
pub struct WorkerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self { host: "0.0.0.0".into(), port: 8080 }
    }
}

/// Configuration for the proxy
#[derive(Serialize, Deserialize)]
pub struct ProxyConfig {
    pub host: String,
    pub port: u16,
    pub timeout_secs: u64,
    pub connection_timeout_secs: u64,
    pub max_queue_items: usize,
    pub max_retries_per_request: usize,
    pub max_req_per_sec: isize,
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
        }
    }
}

/// Root CLI struct
#[derive(Parser, Debug)]
#[clap(
    name = "Miden",
    about = "Miden transaction prover CLI",
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
    /// Creates a config file.
    Init(Init),
    /// Starts the workers defined in the config file.
    StartWorker(StartWorker),
    /// Starts the proxy defined in the config file.
    StartProxy(StartProxy),
    /// Gracefully restart the proxy.
    RestartProxy,
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
            Command::RestartProxy => {
                // Gracefully restart the proxy
                todo!()
            },
        }
    }
}
