extern crate alloc;

use figment::{
    providers::{Format, Toml},
    Figment,
};
use proxy::WorkerLoadBalancer;
use serde::{Deserialize, Serialize};
use tracing::info;
mod api;
mod proxy;
use alloc::string::String;
use std::{fs::File, io::Write};

use clap::Parser;
use pingora::prelude::LoadBalancer;
use tokio::{net::TcpListener, task};
use tokio_stream::wrappers::TcpListenerStream;
pub(crate) mod generated;
use crate::api::RpcListener;

#[cfg(feature = "async")]
mod prover;
use pingora::{apps::HttpServerOptions, prelude::Opt, server::Server};
use pingora_proxy::http_proxy_service;
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

#[derive(Default, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Default, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub host: String,
    pub port: u16,
}

/// Root CLI struct
#[derive(Parser, Debug)]
#[clap(name = "Miden", about = "Miden client", version, rename_all = "kebab-case")]
pub struct Cli {
    #[clap(subcommand)]
    action: Command,
}

/// CLI actions
#[derive(Debug, Parser)]
pub enum Command {
    /// Initialize the CLI and creates a config file.
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
            Command::StartWorker(worker_init) => {
                let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {:?}", e))?;
                rt.block_on(worker_init.execute())
            }
            Command::StartProxy(proxy_init) => {
                proxy_init.execute()
            }
            Command::Init(init) => {
                // Init does not require async, so run directly
                init.execute()
            }
            Command::RestartProxy => {
                // You can implement this in a similar way if needed.
                todo!()
            }
        }
    }
}

/// Initializes the CLI and creates a config file.
#[derive(Debug, Parser)]
pub struct Init;

impl Init {
    pub fn execute(&self) -> Result<(), String> {
        let mut current_dir = std::env::current_dir().map_err(|err| err.to_string())?;
        current_dir.push(PROVER_SERVICE_CONFIG_FILE_NAME);

        if current_dir.exists() {
            return Err(format!(
                "The file \"{}\" already exists in the working directory.",
                PROVER_SERVICE_CONFIG_FILE_NAME
            )
            .to_string());
        }

        let cli_config = CliConfig::default();

        let config_as_toml_string = toml::to_string_pretty(&cli_config)
            .map_err(|err| format!("Error formatting config: {err}"))?;

        let mut file_handle = File::options()
            .write(true)
            .create_new(true)
            .open(&current_dir)
            .map_err(|err| format!("Error opening the file: {err}"))?;

        file_handle
            .write(config_as_toml_string.as_bytes())
            .map_err(|err| format!("Error writing to file: {err}"))?;

        println!("Config file successfully created at: {:?}", current_dir);

        Ok(())
    }
}

/// Starts the workers defined in the config file.
#[derive(Debug, Parser)]
pub struct StartWorker;

impl StartWorker {
    pub async fn execute(&self) -> Result<(), String> {
        let cli_config = load_config_from_file()?;
        let workers_addrs = cli_config
            .workers
            .iter()
            .map(|worker| format!("{}:{}", worker.host, worker.port))
            .collect::<Vec<String>>();

        let mut handles = Vec::new();

        for worker_addr in workers_addrs {
            let handle = task::spawn(async move {
                let rpc = RpcListener::new(
                    TcpListener::bind(&worker_addr).await.map_err(|err| err.to_string())?,
                );

                info!(
                    "Server listening on {}",
                    rpc.listener.local_addr().map_err(|err| err.to_string())?
                );

                tonic::transport::Server::builder()
                    .accept_http1(true)
                    .add_service(tonic_web::enable(rpc.api_service))
                    .serve_with_incoming(TcpListenerStream::new(rpc.listener))
                    .await
                    .map_err(|err| err.to_string())
            });

            handles.push(handle);
        }

        // Wait for all server tasks to complete
        for handle in handles {
            handle.await.map_err(|err| err.to_string())??;
        }

        Ok(())
    }
}

/// Starts the proxy defined in the config file.
#[derive(Debug, Parser)]
pub struct StartProxy;

impl StartProxy {
    pub fn execute(&self) -> Result<(), String> {
        let mut server = Server::new(Some(Opt::default())).expect("Failed to create server");
        server.bootstrap();

        let cli_config = load_config_from_file()?;

        let workers = cli_config
            .workers
            .iter()
            .map(|worker| format!("{}:{}", worker.host, worker.port));

        let workers = LoadBalancer::try_from_iter(workers).expect("PROVER_WORKERS is invalid");

        // Set up the load balancer
        let mut lb = http_proxy_service(&server.configuration, WorkerLoadBalancer::new(workers));

        let proxy_host = cli_config.proxy.host;
        let proxy_port = cli_config.proxy.port.to_string();
        lb.add_tcp(format!("{}:{}", proxy_host, proxy_port).as_str());
        let logic = lb.app_logic_mut().expect("No app logic found");
        let mut http_server_options = HttpServerOptions::default();

        // Enable HTTP/2 for plaintext
        http_server_options.h2c = true;
        logic.server_options = Some(http_server_options);

        server.add_service(lb);

        // Spawn a blocking task to run `run_forever` so it does not interfere with the async
        // runtime.
        // tokio::task::spawn_blocking(|| {
        //         server.run_forever()
        //     }
        //     )
        //     .await
        //     .map_err(|e| format!("Failed to spawn blocking server task: {:?}", e))?;


        server.run_forever();

        Ok(())
    }
}

/// Loads config file from current directory and default filename and returns it
///
/// This function will look for the configuration file at the provided path. If the path is
/// relative, searches in parent directories all the way to the root as well.
pub(crate) fn load_config_from_file() -> Result<CliConfig, String> {
    let mut current_dir = std::env::current_dir().map_err(|err| err.to_string())?;
    current_dir.push(PROVER_SERVICE_CONFIG_FILE_NAME);
    let config_path = current_dir.as_path();

    Figment::from(Toml::file(config_path))
        .extract()
        .map_err(|err| format!("Failed to load {} config file: {err}", config_path.display()))
}
