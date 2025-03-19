use clap::Parser;
use pingora::{
    apps::HttpServerOptions,
    lb::Backend,
    prelude::{Opt, background_service},
    server::Server,
    services::listening::Service,
};
use pingora_proxy::http_proxy_service;
use tracing::warn;

use super::ProxyConfig;
use crate::{
    error::ProvingServiceError,
    proxy::{LoadBalancer, LoadBalancerState, update_workers::LoadBalancerUpdateService},
    utils::MIDEN_PROVING_SERVICE,
};

/// Starts the proxy.
///
/// Example: `miden-proving-service start-proxy 0.0.0.0:8080 127.0.0.1:9090`
#[derive(Debug, Parser)]
pub struct StartProxy {
    /// List of workers as host:port strings.
    ///
    /// Example: `127.0.0.1:8080 192.168.1.1:9090`
    #[clap(value_name = "WORKERS")]
    workers: Vec<String>,
    /// Proxy configurations.
    #[clap(flatten)]
    proxy_config: ProxyConfig,
}

impl StartProxy {
    /// Starts the proxy using the configuration defined in the command.
    ///
    /// This method will start a proxy with each worker passed as command argument as a backend,
    /// using the configurations passed as options for the commands or the equivalent environmental
    /// variables.
    ///
    /// # Errors
    /// Returns an error in the following cases:
    /// - The backend cannot be created.
    /// - The Pingora configuration fails.
    /// - The server cannot be started.
    #[tracing::instrument(target = MIDEN_PROVING_SERVICE, name = "proxy:execute")]
    pub async fn execute(&self) -> Result<(), String> {
        let mut server = Server::new(Some(Opt::default())).map_err(|err| err.to_string())?;
        server.bootstrap();

        println!("Starting proxy with workers: {:?}", self.workers);

        let workers = self
            .workers
            .iter()
            .map(|worker| Backend::new(worker).map_err(ProvingServiceError::BackendCreationFailed))
            .collect::<Result<Vec<Backend>, ProvingServiceError>>()?;

        if workers.is_empty() {
            warn!("Starting the proxy without any workers");
        }

        let worker_lb = LoadBalancerState::new(workers, &self.proxy_config).await?;

        let health_check_service = background_service("health_check", worker_lb);

        let worker_lb = health_check_service.task();

        let updater_service = LoadBalancerUpdateService::new(worker_lb.clone());

        let mut update_workers_service =
            Service::new("update_workers".to_string(), updater_service);
        update_workers_service.add_tcp(
            format!("{}:{}", self.proxy_config.host.clone(), self.proxy_config.workers_update_port)
                .as_str(),
        );

        // Set up the load balancer
        let mut lb = http_proxy_service(&server.configuration, LoadBalancer(worker_lb));

        lb.add_tcp(format!("{}:{}", &self.proxy_config.host, self.proxy_config.port).as_str());
        let logic = lb
            .app_logic_mut()
            .ok_or(ProvingServiceError::PingoraConfigFailed("app logic not found".to_string()))?;
        let mut http_server_options = HttpServerOptions::default();

        // Enable HTTP/2 for plaintext
        http_server_options.h2c = true;
        logic.server_options = Some(http_server_options);

        // Enable Prometheus metrics
        let mut prometheus_service_http =
            pingora::services::listening::Service::prometheus_http_service();
        prometheus_service_http.add_tcp(
            format!(
                "{}:{}",
                self.proxy_config.metrics_config.prometheus_host,
                self.proxy_config.metrics_config.prometheus_port
            )
            .as_str(),
        );

        server.add_service(prometheus_service_http);
        server.add_service(health_check_service);
        server.add_service(update_workers_service);
        server.add_service(lb);
        tokio::task::spawn_blocking(|| server.run_forever())
            .await
            .map_err(|err| err.to_string())?;

        Ok(())
    }
}
