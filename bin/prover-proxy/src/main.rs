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

fn main() {
    tracing_subscriber::fmt().init();

    let mut server = Server::new(Some(Opt::default())).unwrap();
    server.bootstrap();
    let upstreams = LoadBalancer::try_from_iter(["0.0.0.0:8080", "0.0.0.0:50051"]).unwrap();

    // Set health check
    // let hc = TcpHealthCheck::new();
    // upstreams.set_health_check(hc);
    // upstreams.health_check_frequency = Some(Duration::from_secs(1));
    // Set background service
    // let background = background_service("health check", upstreams);
    // let upstreams = background.task();

    // Set load balancer
    let mut lb = http_proxy_service(&server.configuration, LB(upstreams.into()));
    lb.add_tcp("0.0.0.0:6188");

    let mut logic = lb.app_logic_mut().unwrap();
    let mut http_server_options = HttpServerOptions::default();
    http_server_options.h2c = true;
    logic.server_options = Some(http_server_options);

    // let mut tls_settings =
    //     pingora::listeners::TlsSettings::intermediate("cert/localhost.crt", "cert/localhost.key")
    //         .unwrap();

    // tls_settings.enable_h2();

    // lb.add_tls_with_settings("0.0.0.0:8000", None, tls_settings);

    // let rate = Rate
    // server.add_service(background);
    server.add_service(lb);
    server.run_forever();
}

pub struct LB(Arc<LoadBalancer<RoundRobin>>);

impl LB {
    pub fn get_request_appid(&self, session: &mut Session) -> Option<String> {
        match session.req_header().headers.get("appid").map(|v| v.to_str()) {
            None => None,
            Some(v) => match v {
                Ok(v) => Some(v.to_string()),
                Err(_) => None,
            },
        }
    }
}

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
        http_peer.get_mut_peer_options().unwrap().alpn = ALPN::H2;

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
        let appid = match self.get_request_appid(session) {
            None => return Ok(false), // no client appid found, skip rate limiting
            Some(addr) => addr,
        };

        // retrieve the current window requests
        let curr_window_requests = RATE_LIMITER.observe(&appid, 1);
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
