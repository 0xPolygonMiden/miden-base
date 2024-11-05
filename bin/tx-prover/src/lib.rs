extern crate alloc;

use clap::Parser;
use commands::{init::Init, proxy::StartProxy, worker::StartWorker};
use serde::{Deserialize, Serialize};
pub mod api;
mod proxy;
use alloc::string::String;

mod commands;
pub mod generated;
mod utils;

#[cfg(feature = "async")]
mod prover;
#[cfg(feature = "async")]
pub use prover::RemoteTransactionProver;

/// Contains the protobuf definitions
pub const PROTO_MESSAGES: &str = include_str!("../proto/api.proto");

/// Name of the configuration file
pub const PROVER_SERVICE_CONFIG_FILE_NAME: &str = "miden-prover-service.toml";

/// ERRORS
/// ===============================================================================================

#[derive(Debug)]
pub enum RemoteTransactionProverError {
    /// Indicates that the provided gRPC server endpoint is invalid.
    InvalidEndpoint(String),

    /// Indicates that the connection to the server failed.
    ConnectionFailed(String),
}

impl std::fmt::Display for RemoteTransactionProverError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RemoteTransactionProverError::InvalidEndpoint(endpoint) => {
                write!(f, "Invalid endpoint: {}", endpoint)
            },
            RemoteTransactionProverError::ConnectionFailed(endpoint) => {
                write!(f, "Failed to connect to transaction prover at: {}", endpoint)
            },
        }
    }
}

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
#[derive(Default, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub host: String,
    pub port: u16,
}

/// Configuration for the proxy
#[derive(Default, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub host: String,
    pub port: u16,
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
                // You can implement this in a similar way if needed.
                todo!()
            },
        }
    }
}
