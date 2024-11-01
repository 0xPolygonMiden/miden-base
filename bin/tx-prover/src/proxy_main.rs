use pingora::{apps::HttpServerOptions, lb::LoadBalancer, prelude::Opt, server::Server};
use pingora_proxy::http_proxy_service;
mod proxy;

fn main() {
    tracing_subscriber::fmt().init();

    let mut server = Server::new(Some(Opt::default())).expect("Failed to create server");
    server.bootstrap();

    let workers =
        std::env::var("PROVER_WORKERS").expect("PROVER_WORKERS environment variable not set");
    let workers =
        LoadBalancer::try_from_iter(workers.split(",")).expect("PROVER_WORKERS is invalid");

    // Set load balancer
    let mut lb = http_proxy_service(&server.configuration, proxy::WorkerLoadBalancer::new(workers));

    let proxy_host = std::env::var("PROXY_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let proxy_port = std::env::var("PROXY_PORT").unwrap_or_else(|_| "6188".to_string());
    lb.add_tcp(format!("{}:{}", proxy_host, proxy_port).as_str());

    let logic = lb.app_logic_mut().expect("No app logic found");
    let mut http_server_options = HttpServerOptions::default();

    // Enable HTTP/2 for plaintext
    http_server_options.h2c = true;
    logic.server_options = Some(http_server_options);

    server.add_service(lb);
    server.run_forever();
}
