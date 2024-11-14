use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use once_cell::sync::Lazy;
use pingora::{
    http::ResponseHeader,
    lb::Backend,
    prelude::{LoadBalancer as PingoraLoadBalancer, *},
    upstreams::peer::{Peer, ALPN},
};
use pingora_core::{upstreams::peer::HttpPeer, Result};
use pingora_limits::rate::Rate;
use pingora_proxy::{ProxyHttp, Session};
use tokio::sync::RwLock;
use tracing::error;

use crate::commands::ProxyConfig;

const RESOURCE_EXHAUSTED_CODE: u16 = 8;

/// Load balancer that uses a round robin strategy
pub struct LoadBalancer {
    lb: Arc<PingoraLoadBalancer<RoundRobin>>,
    timeout_secs: Duration,
    connection_timeout_secs: Duration,
    max_queue_items: usize,
    max_retries_per_request: usize,
    max_req_per_sec: isize,
}

impl LoadBalancer {
    pub fn new(workers: PingoraLoadBalancer<RoundRobin>, config: &ProxyConfig) -> Self {
        Self {
            lb: Arc::new(workers),
            timeout_secs: Duration::from_secs(config.timeout_secs),
            connection_timeout_secs: Duration::from_secs(config.connection_timeout_secs),
            max_queue_items: config.max_queue_items,
            max_retries_per_request: config.max_retries_per_request,
            max_req_per_sec: config.max_req_per_sec,
        }
    }

    /// Create a 429 response for too many requests
    pub async fn create_too_many_requests_response(
        session: &mut Session,
        max_request_per_second: isize,
    ) -> Result<bool> {
        // Rate limited, return 429
        let mut header = ResponseHeader::build(429, None)?;
        header.insert_header("X-Rate-Limit-Limit", max_request_per_second.to_string())?;
        header.insert_header("X-Rate-Limit-Remaining", "0")?;
        header.insert_header("X-Rate-Limit-Reset", "1")?;
        session.set_keepalive(None);
        session.write_response_header(Box::new(header), true).await?;
        Ok(true)
    }

    /// Create a 503 response for a full queue
    pub async fn create_queue_full_response(session: &mut Session) -> Result<bool> {
        // Set grpc-message header to "Too many requests in the queue"
        // This is meant to be used by a Tonic interceptor to return a gRPC error
        let mut header = ResponseHeader::build(503, None)?;
        header.insert_header("grpc-message", "Too many requests in the queue".to_string())?;
        header.insert_header("grpc-status", RESOURCE_EXHAUSTED_CODE)?;
        session.set_keepalive(None);
        session.write_response_header(Box::new(header.clone()), true).await?;

        let mut error = Error::new(ErrorType::HTTPStatus(503))
            .more_context("Too many requests in the queue")
            .into_in();
        error.set_cause("Too many requests in the queue");

        session.write_response_header(Box::new(header), false).await?;
        Err(error)
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

/// Shared state. It is a map of workers to a vector of request IDs
static QUEUES: Lazy<RwLock<HashMap<Backend, Vec<String>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Custom context for the request/response lifecycle
/// We use this context to keep track of the number of tries for a request.
pub struct TriesCounter {
    tries: usize,
}

/// Implements load-balancing of incoming requests across a pool of workers.
///
/// At the backend-level, a request lifecycle works as follows:
/// - When a new requests arrives, [LoadBalancer::request_filter()] method is called. In this method
///   we apply IP-based rate-limiting to the request and assign a unique ID to it.
/// - Next, the [Self::upstream_peer()] method is called. We use it to figure out which worker will
///   process the request. Inside `upstream_peer()`, we pick a worker in a round-robin fashion and
///   add the request to the queue of requests for that worker. Once the request gets to the front
///   of the queue, we forward it to the worker. This step is also in charge of assinging the
///   timeouts and enabling HTTP/2. Finally, we establish a connection with the worker.
/// - Before sending the request to the upstream server and if the connection succeed, the
///   [Self::upstream_request_filter()] method is called. In this method, we ensure that the correct
///   headers are forwarded for gRPC requests.
/// - If the connection fails, the [Self::fail_to_connect()] method is called. In this method, we
///   retry the request [self.max_retries_per_request] times.
/// - Once the worker processes the request (either successfully or with a failure),
///   [Self::logging()] method is called. In this method, we remove the request from the worker's
///   queue, allowing the worker to process the next request.
#[async_trait]
impl ProxyHttp for LoadBalancer {
    type CTX = TriesCounter;
    fn new_ctx(&self) -> Self::CTX {
        TriesCounter { tries: 0 }
    }

    /// Decide whether to filter the request or not.
    ///
    /// Here we apply IP-based rate-limiting to the request. We assign a unique ID to the request
    /// and check if the current window requests exceed the maximum allowed requests per second.
    ///
    /// If the request is rate-limited, we return a 429 response. Otherwise, we return false.
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

        if curr_window_requests > self.max_req_per_sec {
            return Self::create_too_many_requests_response(session, self.max_req_per_sec).await;
        };
        Ok(false)
    }

    /// Returns [HttpPeer] corresponding to the worker that will handle the current request.
    ///
    /// Here we select the next worker from the pool in a round-robin fashion. We then add the
    /// request to the worker's queue and wait until it gets to the front of it. Then, we construct
    /// and return the [HttpPeer]. The peer is configured with timeouts, and HTTP/2.
    ///
    /// Note that the request is not removed from the queue here. It will be returned later in
    /// [Self::logging()] once the worker processes the it.
    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        if ctx.tries > 0 {
            Self::remove_request_from_queue(Self::get_request_id(session)?).await;
        }

        // Select the worker in a round-robin fashion
        let worker = self.lb.select(b"", 256).ok_or(Error::new_str("Worker not found"))?;

        // Read request ID from headers
        let request_id = {
            let id = Self::get_request_id(session)?;
            id.to_string()
        };

        // Enqueue the request ID in the worker queue
        // We use a new scope to release the lock after the operation
        {
            let mut ctx_guard = QUEUES.write().await;
            let worker_queue = ctx_guard.entry(worker.clone()).or_insert_with(Vec::new);

            // Limit queue length
            if worker_queue.len() >= self.max_queue_items {
                Self::create_queue_full_response(session).await?;
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
        peer_opts.total_connection_timeout = Some(self.timeout_secs);
        peer_opts.connection_timeout = Some(self.connection_timeout_secs);
        peer_opts.read_timeout = Some(self.timeout_secs);
        peer_opts.write_timeout = Some(self.timeout_secs);
        peer_opts.idle_timeout = Some(self.timeout_secs);

        // Enable HTTP/2
        peer_opts.alpn = ALPN::H2;

        let peer = Box::new(http_peer);
        Ok(peer)
    }

    /// Applies the necessary filters to the request before sending it to the upstream server.
    ///
    /// Here we ensure that the correct headers are forwarded for gRPC requests.
    ///
    /// This method is called right after [Self::upstream_peer()] returns a [HttpPeer] and a
    /// connection is established with the worker.
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

    /// Retry the request if the connection fails.
    fn fail_to_connect(
        &self,
        _session: &mut Session,
        _peer: &HttpPeer,
        ctx: &mut Self::CTX,
        mut e: Box<Error>,
    ) -> Box<Error> {
        if ctx.tries > self.max_retries_per_request {
            return e;
        }
        ctx.tries += 1;
        e.set_retry(true);
        e
    }

    /// Logs the request lifecycle in case that an error happened and removes the request from the
    /// worker queue.
    ///
    /// This method is the last one in the request lifecycle, no matter if the request was
    /// processed or not.
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
