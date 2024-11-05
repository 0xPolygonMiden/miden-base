use clap::Parser;
use pingora::{apps::HttpServerOptions, lb::LoadBalancer, prelude::Opt, server::Server};
use pingora_proxy::http_proxy_service;

use crate::{proxy::WorkerLoadBalancer, utils::load_config_from_file};

/// Starts the proxy defined in the config file.
#[derive(Debug, Parser)]
pub struct StartProxy;

impl StartProxy {
    pub fn execute(&self) -> Result<(), String> {
        tracing_subscriber::fmt::init();
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
        server.run_forever();
    }
}
