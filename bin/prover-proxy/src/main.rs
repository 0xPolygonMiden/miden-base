use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use once_cell::sync::Lazy;
use pingora::{
    apps::HttpServerOptions,
    http::ResponseHeader,
    prelude::*,
    upstreams::peer::{Peer, ALPN},
};
use pingora_core::{prelude::Opt, server::Server, upstreams::peer::HttpPeer, Result};
use pingora_limits::rate::Rate;
use pingora_proxy::{ProxyHttp, Session};

const TIMEOUT_SECS: Option<Duration> = Some(Duration::from_secs(30));

fn main() {
    tracing_subscriber::fmt().init();

    let mut server = Server::new(Some(Opt::default())).unwrap();
    server.bootstrap();
    let upstreams = LoadBalancer::try_from_iter(["0.0.0.0:8080", "0.0.0.0:50051"]).unwrap();

    // Set load balancer
    let mut lb = http_proxy_service(&server.configuration, LB(upstreams.into()));
    lb.add_tcp("0.0.0.0:6188");

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
static MAX_REQ_PER_SEC: isize = 1;

#[async_trait]
impl ProxyHttp for LB {
    type CTX = ();

    fn new_ctx(&self) {}

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let upstream = self.0.select(b"", 256).unwrap();
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
}
