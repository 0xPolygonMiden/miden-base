use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use once_cell::sync::Lazy;
use pingora::{
    http::ResponseHeader,
    lb::Backend,
    prelude::*,
    upstreams::peer::{Peer, ALPN},
};
use pingora_core::{upstreams::peer::HttpPeer, Result};
use pingora_limits::rate::Rate;
use pingora_proxy::{ProxyHttp, Session};
use tokio::sync::RwLock;
use tracing::error;

/// Timeout duration for the requests
const TIMEOUT_SECS: Option<Duration> = Some(Duration::from_secs(100));

/// Timeout duration for the connection
const CONNECTION_TIMEOUT_SECS: Option<Duration> = Some(Duration::from_secs(10));

/// Maximum number of items per queue
const MAX_QUEUE_ITEMS: usize = 10;

/// Maximum number of retries per request
const MAX_RETRIES_PER_REQUEST: usize = 1;

/// Load balancer that uses a round robin strategy
pub struct WorkerLoadBalancer(Arc<LoadBalancer<RoundRobin>>);

impl WorkerLoadBalancer {
    pub fn new(workers: LoadBalancer<RoundRobin>) -> Self {
        Self(Arc::new(workers))
    }

    /// Create a 429 response for too many requests
    pub async fn create_too_many_requests_response(session: &mut Session) -> Result<bool> {
        // Rate limited, return 429
        let mut header = ResponseHeader::build(429, None)?;
        header.insert_header("X-Rate-Limit-Limit", MAX_REQ_PER_SEC.to_string())?;
        header.insert_header("X-Rate-Limit-Remaining", "0")?;
        header.insert_header("X-Rate-Limit-Reset", "1")?;
        session.set_keepalive(None);
        session.write_response_header(Box::new(header), true).await?;
        Ok(true)
    }

    /// Remove the request ID from the corresponding worker queue
    pub async fn remove_request_from_queue(request_id: &str) {
        let mut ctx_guard = QUEUES.write().await;

        // Get the worker by checking each queue
        let worker = ctx_guard
            .iter()
            .find(|(_, queue)| queue.contains(&request_id.to_string()))
            .map(|(worker, _)| worker.clone())
            .expect("Worker not found");

        // Remove the request ID from the queue for the specific worker
        if let Some(worker_queue) = ctx_guard.get_mut(&worker) {
            if !worker_queue.is_empty() {
                worker_queue.remove(0);
            }
        }
    }

    /// Get the request ID from the session headers
    pub fn get_request_id(session: &Session) -> Result<&str> {
        session
            .get_header("X-Request-ID")
            .ok_or(Error::new(ErrorType::InternalError))?
            .to_str()
            .map_err(|_| Error::new(ErrorType::InternalError))
    }
}

/// Rate limiter
static RATE_LIMITER: Lazy<Rate> = Lazy::new(|| Rate::new(Duration::from_secs(1)));

/// Maximum amount of request per second per client
static MAX_REQ_PER_SEC: isize = 5;

/// Shared state. It is a map of workers to a vector of request IDs
static QUEUES: Lazy<RwLock<HashMap<Backend, Vec<String>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Custom context for the request/response lifecycle
/// We use this context to keep track of the number of tries for a request.
pub struct TriesCounter {
    tries: usize,
}

#[async_trait]
/// The [ProxyHttp] trait enables implementing a custom HTTP proxy service.
/// Defined in the [pingora_proxy] crate, this trait provides several methods
/// that correspond to different stages of the request/response lifecycle.
/// Most methods have default implementations, making them optional to override.
/// For a detailed explanation of the request/response cycle, refer to the
/// [official documentation](https://github.com/cloudflare/pingora/blob/main/docs/user_guide/phase.md).
impl ProxyHttp for WorkerLoadBalancer {
    type CTX = TriesCounter;
    fn new_ctx(&self) -> Self::CTX {
        TriesCounter { tries: 0 }
    }

    // The `upstream_peer` method is called when a new upstream connection is required.
    // This method is responsible for selecting a worker from the load balancer and
    // creating a new peer with the selected worker.
    // Here we enqueue the request ID in the worker queue and wait for it to be at the front.
    // If we are retring the request, we remove the request from the queue first.
    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        if ctx.tries > 0 {
            Self::remove_request_from_queue(Self::get_request_id(session)?).await;
        }

        // Select the worker in a round-robin fashion
        let worker = self.0.select(b"", 256).ok_or(Error::new_str("Worker not found"))?;

        // Read request ID from headers
        let request_id = Self::get_request_id(session)?;

        // Enqueue the request ID in the worker queue
        // We use a new scope to release the lock after the operation
        {
            let mut ctx_guard = QUEUES.write().await;
            let worker_queue = ctx_guard.entry(worker.clone()).or_insert_with(Vec::new);

            // Limit queue length to MAX_QUEUE_ITEMS requests
            if worker_queue.len() >= MAX_QUEUE_ITEMS {
                return Err(Error::new_str("Too many requests in the queue"));
            }

            worker_queue.push(request_id.to_string());
        }

        // Wait for the request to be at the front of the queue
        loop {
            // We use a new scope for each iteration to release the lock
            {
                let ctx_guard = QUEUES.read().await;
                if let Some(worker_queue) = ctx_guard.get(&worker) {
                    if worker_queue[0] == request_id {
                        break;
                    }
                } else {
                    return Err(Error::new_str("Worker not found"));
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Set SNI
        let mut http_peer = HttpPeer::new(worker, false, "".to_string());
        let peer_opts =
            http_peer.get_mut_peer_options().ok_or(Error::new(ErrorType::InternalError))?;

        // Timeout settings
        peer_opts.total_connection_timeout = TIMEOUT_SECS;
        peer_opts.connection_timeout = CONNECTION_TIMEOUT_SECS;
        peer_opts.read_timeout = TIMEOUT_SECS;
        peer_opts.write_timeout = TIMEOUT_SECS;
        peer_opts.idle_timeout = TIMEOUT_SECS;

        // Enable HTTP/2
        peer_opts.alpn = ALPN::H2;

        let peer = Box::new(http_peer);
        Ok(peer)
    }

    // The `upstream_request_filter` method is called before sending the request to the upstream
    // server. We use the method to ensure that the correct headers are forwarded for gRPC
    // requests.
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
                upstream_request.insert_header("content-type", "application/grpc")?;
            }
        }

        Ok(())
    }

    // The `request_filter` method is called before processing the request.
    // We use the method to rate limit the requests and add a unique request ID to the headers.
    async fn request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        let client_addr = session.client_addr();
        let user_id = client_addr.map(|addr| addr.to_string());

        // Request ID is a random number
        let request_id = rand::random::<u64>().to_string();
        session.req_header_mut().insert_header("X-Request-ID", request_id)?;

        // Retrieve the current window requests
        let curr_window_requests = RATE_LIMITER.observe(&user_id, 1);

        if curr_window_requests > MAX_REQ_PER_SEC {
            return Self::create_too_many_requests_response(session).await;
        };
        Ok(false)
    }

    // If the connection fails, we retry the request [MAX_RETRIES_PER_REQUEST] times.
    fn fail_to_connect(
        &self,
        _session: &mut Session,
        _peer: &HttpPeer,
        ctx: &mut Self::CTX,
        mut e: Box<Error>,
    ) -> Box<Error> {
        if ctx.tries > MAX_RETRIES_PER_REQUEST {
            return e;
        }
        ctx.tries += 1;
        e.set_retry(true);
        e
    }

    // The `logging` method is called after the request cycle is complete no matter the outcome.
    // We use the method to log errors and remove the completed request from the worker queue.
    async fn logging(&self, session: &mut Session, e: Option<&Error>, _ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        if let Some(e) = e {
            error!("Error: {:?}", e);
        }

        // Get the request ID from the session
        let request_id = Self::get_request_id(session).expect("Request ID not found");

        // Remove the completed request from the worker queue
        // Maybe we can replace this with a read lock and using write only in the moment of the
        // deletion.
        Self::remove_request_from_queue(request_id).await;
    }
}
