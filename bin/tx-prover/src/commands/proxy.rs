use clap::Parser;
use pingora::{apps::HttpServerOptions, lb::Backend, prelude::Opt, server::Server};
use pingora_proxy::http_proxy_service;

use crate::proxy::LoadBalancer;

/// Starts the proxy defined in the config file.
#[derive(Debug, Parser)]
pub struct StartProxy;

impl StartProxy {
    /// Starts the proxy defined in the config file.
    ///
    /// This method will first read the config file to get the list of workers to start. It will
    /// then start a proxy with each worker as a backend.
    pub fn execute(&self) -> Result<(), String> {
        let mut server = Server::new(Some(Opt::default())).expect("Failed to create server");
        server.bootstrap();

        let proxy_config = super::ProxyConfig::load_config_from_file()?;

        let workers = proxy_config
            .workers
            .iter()
            .map(|worker| format!("{}:{}", worker.host, worker.port))
            .map(|worker| Backend::new(&worker).expect("Failed to create backend"))
            .collect::<Vec<Backend>>();

        let worker_lb = LoadBalancer::new(workers, &proxy_config);

        // Set up the load balancer
        let mut lb = http_proxy_service(&server.configuration, worker_lb);

        let proxy_host = proxy_config.host;
        let proxy_port = proxy_config.port.to_string();
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
