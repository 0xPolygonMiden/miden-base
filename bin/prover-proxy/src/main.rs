use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use once_cell::sync::Lazy;
use pingora::{
    apps::HttpServerOptions,
    http::ResponseHeader,
    lb::Backend,
    prelude::*,
    upstreams::peer::{Peer, ALPN},
};
use pingora_core::{prelude::Opt, server::Server, upstreams::peer::HttpPeer, Result};
use pingora_limits::rate::Rate;
use pingora_proxy::{ProxyHttp, Session};
use tokio::sync::RwLock;
use tracing::error;

const TIMEOUT_SECS: Option<Duration> = Some(Duration::from_secs(100));
const MAX_QUEUE_ITEMS: usize = 10;

fn main() {
    tracing_subscriber::fmt().init();

    let mut server = Server::new(Some(Opt::default())).unwrap();
    server.bootstrap();

    let backends =
        std::env::var("PROVER_BACKENDS").expect("PROVER_BACKENDS environment variable not set");
    let upstreams = LoadBalancer::try_from_iter(backends.split(",")).unwrap();

    // Set load balancer
    let mut lb = http_proxy_service(&server.configuration, LB(upstreams.into()));

    let proxy_host = std::env::var("PROXY_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let proxy_port = std::env::var("PROXY_PORT").unwrap_or_else(|_| "6188".to_string());
    lb.add_tcp(format!("{}:{}", proxy_host, proxy_port).as_str());

    let logic = lb.app_logic_mut().unwrap();
    let mut http_server_options = HttpServerOptions::default();
    http_server_options.h2c = true;
    logic.server_options = Some(http_server_options);

    server.add_service(lb);
    server.run_forever();
}

pub struct LB(Arc<LoadBalancer<RoundRobin>>);

// Rate limiter
static RATE_LIMITER: Lazy<Rate> = Lazy::new(|| Rate::new(Duration::from_secs(1)));

// max request per second per client
static MAX_REQ_PER_SEC: isize = 10000;

// Shared state
static QUEUES: Lazy<RwLock<HashMap<Backend, Vec<String>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

#[async_trait]
impl ProxyHttp for LB {
    type CTX = ();

    fn new_ctx(&self) {}

    async fn upstream_peer(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let upstream = self.0.select(b"", 256).unwrap();

        // read request ID from headers
        let request_id = session
            .get_header("X-Request-ID")
            .expect("Request ID not found")
            .to_str()
            .expect("Invalid header value");

        {
            let mut ctx_guard = QUEUES.write().await;
            let backend_queue = ctx_guard.entry(upstream.clone()).or_insert_with(|| Vec::new());

            // Limit queue length to MAX_QUEUE_ITEMS requests
            if backend_queue.len() >= MAX_QUEUE_ITEMS {
                panic!("Too many requests in the queue");
                // return Err(Box::new("Too many requests in the queue".into()));
            }

            backend_queue.push(request_id.to_string());
        }

        // Wait for the request to be at the front of the queue
        loop {
            // We use a new scope for each iteration to release the lock
            {
                let ctx_guard = QUEUES.read().await;
                if let Some(backend_queue) = ctx_guard.get(&upstream) {
                    if backend_queue[0] == request_id {
                        break;
                    }
                } else {
                    // TODO: replace this panic with an error
                    panic!("Upstream not found");
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Set SNI
        let mut http_peer = HttpPeer::new(upstream, false, "".to_string());
        let peer_opts = http_peer.get_mut_peer_options().unwrap();

        // Timeout settings
        peer_opts.total_connection_timeout = TIMEOUT_SECS;
        peer_opts.connection_timeout = TIMEOUT_SECS;
        peer_opts.read_timeout = TIMEOUT_SECS;
        peer_opts.write_timeout = TIMEOUT_SECS;
        peer_opts.idle_timeout = TIMEOUT_SECS;

        peer_opts.alpn = ALPN::H2;

        let peer = Box::new(http_peer);
        Ok(peer)
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()>
    where
        Self::CTX: Send + Sync,
    {
        // Check if it's a gRPC request
        if let Some(content_type) = upstream_request.headers.get("content-type") {
            if content_type == "application/grpc" {
                // Ensure the correct host and gRPC headers are forwarded
                // upstream_request.insert_header("Host", "0.0.0.0").unwrap();
                upstream_request.insert_header("content-type", "application/grpc").unwrap();
            }
        }

        Ok(())
    }

    async fn request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        let client_addr = session.client_addr();
        let user_id = client_addr.map(|addr| addr.to_string());

        // Request ID is a random number
        let request_id = rand::random::<u64>().to_string();
        session
            .req_header_mut()
            .insert_header("X-Request-ID", request_id)
            .expect("Failed to insert header");

        // retrieve the current window requests
        let curr_window_requests = RATE_LIMITER.observe(&user_id, 1);

        if curr_window_requests > MAX_REQ_PER_SEC {
            // rate limited, return 429
            let mut header = ResponseHeader::build(429, None).unwrap();
            header.insert_header("X-Rate-Limit-Limit", MAX_REQ_PER_SEC.to_string()).unwrap();
            header.insert_header("X-Rate-Limit-Remaining", "0").unwrap();
            header.insert_header("X-Rate-Limit-Reset", "1").unwrap();
            session.set_keepalive(None);
            session.write_response_header(Box::new(header), true).await?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn logging(&self, session: &mut Session, e: Option<&Error>, _ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        if let Some(e) = e {
            error!("Error: {:?}", e);
        }

        // Get the request ID from the session
        let request_id = session
            .get_header("X-Request-ID")
            .expect("Request ID not found")
            .to_str()
            .expect("Invalid header value");

        // Remove the completed request from the backend queue
        // Maybe we can replace this with a read lock and using write only in the moment of the
        // deletion.
        let mut ctx_guard = QUEUES.write().await;

        // Get the upstream by checking each backend queue
        let upstream = ctx_guard
            .iter()
            .find(|(_, queue)| queue.contains(&request_id.to_string()))
            .map(|(upstream, _)| upstream.clone())
            .expect("Upstream not found");

        // Remove the request ID from the queue for the specific backend
        if let Some(backend_queue) = ctx_guard.get_mut(&upstream) {
            if !backend_queue.is_empty() {
                backend_queue.remove(0);
            }
        }
    }
}
