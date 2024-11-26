use clap::Parser;
use pingora::{
    apps::HttpServerOptions,
    lb::Backend,
    prelude::Opt,
    server::{configuration::ServerConf, Server},
};
use pingora_proxy::http_proxy_service;
use tracing::info;

use crate::{proxy::LoadBalancer, utils::load_config_from_file};

/// Starts the proxy defined in the config file.
#[derive(Debug, Parser)]
pub struct StartProxy {
    /// Whether this server should try to upgrade from a running old server
    #[clap(short, long, default_value = "false")]
    pub upgrade: bool,
    /// The path to the configuration file.
    ///
    /// See [`ServerConf`] for more details of the configuration file.
    #[clap(short, long, help = "The path to the configuration file.", long_help = None)]
    pub conf: Option<String>,
}

impl StartProxy {
    /// Starts the proxy defined in the config file.
    ///
    /// This method will first read the config file to get the list of workers to start. It will
    /// then start a proxy with each worker as a backend.
    pub fn execute(&self) -> Result<(), String> {
        let mut opts = Opt::default();

        opts.upgrade = self.upgrade;
        opts.conf = self.conf.clone();

        info!("Starting proxy with options: {:?}", opts);
        let confs = opts.conf.as_ref().map_or_else(
            || {
                // options, no conf, generated
                info!("No configuration file provided, generating default configuration");
                ServerConf::new_with_opt_override(&opts).unwrap()
            },
            |_| {
                info!("Loading configuration file");
                // options and conf loaded
                ServerConf::load_yaml_with_opt_override(&opts)
            },
        );

        info!("Starting server with configuration: {:?}", confs);

        let mut server = Server::new(Some(opts)).expect("Failed to create server");
        server.bootstrap();

        let proxy_config = load_config_from_file()?;

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
