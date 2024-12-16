use clap::Parser;
use pingora::{
    apps::HttpServerOptions,
    lb::Backend,
    prelude::{background_service, Opt},
    server::Server,
};
use pingora_proxy::http_proxy_service;
use tracing::warn;

use crate::proxy::{LoadBalancer, LoadBalancerState};

/// Starts the proxy defined in the config file.
///
/// Example: `miden-tx-prover start-proxy 0.0.0.0:8080 127.0.0.1:9090`
#[derive(Debug, Parser)]
pub struct StartProxy {
    /// List of workers as host:port strings.
    ///
    /// Example: `127.0.0.1:8080 192.168.1.1:9090`
    #[clap(value_name = "WORKERS")]
    workers: Vec<String>,
}

impl StartProxy {
    /// Starts the proxy defined in the config file.
    ///
    /// This method will first read the config file to get the parameters for the proxy. It will
    /// then start a proxy with each worker passed as command argument as a backend.
    pub async fn execute(&self) -> Result<(), String> {
        let mut server = Server::new(Some(Opt::default())).map_err(|err| err.to_string())?;
        server.bootstrap();

        let proxy_config = super::ProxyConfig::load_config_from_file()?;

        let workers = self
            .workers
            .iter()
            .map(|worker| Backend::new(worker).map_err(|err| err.to_string()))
            .collect::<Result<Vec<Backend>, String>>()?;

        if workers.is_empty() {
            warn!("Starting the proxy without any workers");
        }

        let worker_lb = LoadBalancerState::new(workers, &proxy_config).await?;

        let health_check_service = background_service("health_check", worker_lb);
        let worker_lb = health_check_service.task();

        // Set up the load balancer
        let mut lb = http_proxy_service(&server.configuration, LoadBalancer(worker_lb));

        let proxy_host = proxy_config.host;
        let proxy_port = proxy_config.port.to_string();
        lb.add_tcp(format!("{}:{}", proxy_host, proxy_port).as_str());
        let logic = lb.app_logic_mut().ok_or("Failed to get app logic")?;
        let mut http_server_options = HttpServerOptions::default();

        // Enable HTTP/2 for plaintext
        http_server_options.h2c = true;
        logic.server_options = Some(http_server_options);

        server.add_service(health_check_service);
        server.add_service(lb);
        tokio::task::spawn_blocking(|| server.run_forever())
            .await
            .map_err(|err| err.to_string())?;

        Ok(())
    }
}
