use std::{collections::VecDeque, sync::Arc, time::Duration};

use async_trait::async_trait;
use once_cell::sync::Lazy;
use pingora::{
    lb::Backend,
    prelude::*,
    upstreams::peer::{Peer, ALPN},
};
use pingora_core::{upstreams::peer::HttpPeer, Result};
use pingora_limits::rate::Rate;
use pingora_proxy::{ProxyHttp, Session};
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    commands::ProxyConfig,
    utils::{create_queue_full_response, create_too_many_requests_response},
};

// LOAD BALANCER
// ================================================================================================

/// Load balancer that uses a round robin strategy
pub struct LoadBalancer {
    available_workers: Arc<RwLock<Vec<Backend>>>,
    timeout_secs: Duration,
    connection_timeout_secs: Duration,
    max_queue_items: usize,
    max_retries_per_request: usize,
    max_req_per_sec: isize,
    available_workers_polling_time: Duration,
}

impl LoadBalancer {
    /// Create a new load balancer
    pub fn new(workers: Vec<Backend>, config: &ProxyConfig) -> Self {
        Self {
            available_workers: Arc::new(RwLock::new(workers)),
            timeout_secs: Duration::from_secs(config.timeout_secs),
            connection_timeout_secs: Duration::from_secs(config.connection_timeout_secs),
            max_queue_items: config.max_queue_items,
            max_retries_per_request: config.max_retries_per_request,
            max_req_per_sec: config.max_req_per_sec,
            available_workers_polling_time: Duration::from_millis(
                config.available_workers_polling_time_ms,
            ),
        }
    }

    /// Gets an available worker and removes it from the list of available workers.
    ///
    /// If no worker is available, it will return None.
    pub async fn pop_available_worker(&self) -> Option<Backend> {
        self.available_workers.write().await.pop()
    }

    /// Adds the provided worker to the list of available workers.
    ///
    /// # Panics
    /// Panics if the provided worker is already in the list of available workers.
    pub async fn add_available_worker(&self, worker: Backend) {
        let mut available_workers = self.available_workers.write().await;
        assert!(!available_workers.contains(&worker), "Worker already available");
        info!("Worker {} is now available", worker.addr);
        available_workers.push(worker);
    }
}

/// Rate limiter
static RATE_LIMITER: Lazy<Rate> = Lazy::new(|| Rate::new(Duration::from_secs(1)));

// REQUEST QUEUE
// ================================================================================================

/// Request queue holds the list of requests that are waiting to be processed by the workers.
/// It is used to keep track of the order of the requests to then assign them to the workers.
pub struct RequestQueue {
    queue: RwLock<VecDeque<Uuid>>,
}

impl RequestQueue {
    /// Create a new empty request queue
    pub fn new() -> Self {
        Self { queue: RwLock::new(VecDeque::new()) }
    }

    /// Get the length of the queue
    pub async fn len(&self) -> usize {
        self.queue.read().await.len()
    }

    /// Enqueue a request
    pub async fn enqueue(&self, request_id: Uuid) {
        let mut queue = self.queue.write().await;
        queue.push_back(request_id);
    }

    /// Dequeue a request
    pub async fn dequeue(&self) -> Option<Uuid> {
        let mut queue = self.queue.write().await;
        queue.pop_front()
    }

    /// Peek at the first request in the queue
    pub async fn peek(&self) -> Option<Uuid> {
        let queue = self.queue.read().await;
        queue.front().copied()
    }
}

/// Shared state. It keeps track of the order of the requests to then assign them to the workers.
static QUEUE: Lazy<RequestQueue> = Lazy::new(RequestQueue::new);

// REQUEST CONTEXT
// ================================================================================================

/// Custom context for the request/response lifecycle
/// We use this context to keep track of the number of tries for a request, the unique ID for the
/// request, and the worker that will process the request.
pub struct RequestContext {
    /// Number of tries for the request
    tries: usize,
    /// Unique ID for the request
    request_id: Uuid,
    /// Worker that will process the request
    worker: Option<Backend>,
}

impl RequestContext {
    /// Create a new request context
    fn new() -> Self {
        Self {
            tries: 0,
            request_id: Uuid::new_v4(),
            worker: None,
        }
    }

    /// Set the worker that will process the request
    fn set_worker(&mut self, worker: Backend) {
        self.worker = Some(worker);
    }
}

/// Implements load-balancing of incoming requests across a pool of workers.
///
/// At the backend-level, a request lifecycle works as follows:
/// - When a new requests arrives, [LoadBalancer::request_filter()] method is called. In this method
///   we apply IP-based rate-limiting to the request and check if the request queue is full.
/// - Next, the [Self::upstream_peer()] method is called. We use it to figure out which worker will
///   process the request. Inside `upstream_peer()`, we add the request to the queue of requests.
///   Once the request gets to the front of the queue, we forward it to an available worker. This
///   step is also in charge of setting the SNI, timeouts, and enabling HTTP/2. Finally, we
///   establish a connection with the worker.
/// - Before sending the request to the upstream server and if the connection succeed, the
///   [Self::upstream_request_filter()] method is called. In this method, we ensure that the correct
///   headers are forwarded for gRPC requests.
/// - If the connection fails, the [Self::fail_to_connect()] method is called. In this method, we
///   retry the request [self.max_retries_per_request] times.
/// - Once the worker processes the request (either successfully or with a failure),
///   [Self::logging()] method is called. In this method, we log the request lifecycle and set the
///   worker as available.
#[async_trait]
impl ProxyHttp for LoadBalancer {
    type CTX = RequestContext;
    fn new_ctx(&self) -> Self::CTX {
        RequestContext::new()
    }

    /// Decide whether to filter the request or not.
    ///
    /// Here we apply IP-based rate-limiting to the request. We also check if the queue is full.
    ///
    /// If the request is rate-limited, we return a 429 response. Otherwise, we return false.
    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        let client_addr = session.client_addr();
        let user_id = client_addr.map(|addr| addr.to_string());

        // Retrieve the current window requests
        let curr_window_requests = RATE_LIMITER.observe(&user_id, 1);

        // Rate limit the request
        if curr_window_requests > self.max_req_per_sec {
            return create_too_many_requests_response(session, self.max_req_per_sec).await;
        };

        let queue_len = QUEUE.len().await;

        info!("New request with ID: {}", ctx.request_id);
        info!("Queue length: {}", queue_len);

        // Check if the queue is full
        if queue_len >= self.max_queue_items {
            return create_queue_full_response(session).await;
        }

        Ok(false)
    }

    /// Returns [HttpPeer] corresponding to the worker that will handle the current request.
    ///
    /// Here we enqueue the request and wait for it to be at the front of the queue and a worker
    /// becomes available, then we dequeue the request and process it. We then set the SNI,
    /// timeouts, and enable HTTP/2.
    ///
    /// Note that the request will be assigned a worker here, and the worker will be removed from
    /// the list of available workers once it reaches the [Self::logging] method.
    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let request_id = ctx.request_id;

        // Add the request to the queue.
        QUEUE.enqueue(request_id).await;

        // Wait for the request to be at the front of the queue
        loop {
            // The request is at the front of the queue.
            if QUEUE.peek().await.expect("Queue should not be empty") != request_id {
                continue;
            }

            // Check if there is an available worker
            if let Some(worker) = self.pop_available_worker().await {
                info!("Worker {} picked up the request with ID: {}", worker.addr, request_id);
                ctx.set_worker(worker);
                break;
            }
            info!("All workers are busy");
            tokio::time::sleep(self.available_workers_polling_time).await;
        }

        // Remove the request from the queue
        QUEUE.dequeue().await;

        // Set SNI
        let mut http_peer =
            HttpPeer::new(ctx.worker.clone().expect("Failed to get worker"), false, "".to_string());
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

    /// Logs the request lifecycle in case that an error happened and sets the worker as available.
    ///
    /// This method is the last one in the request lifecycle, no matter if the request was
    /// processed or not.
    async fn logging(&self, _session: &mut Session, e: Option<&Error>, ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        if let Some(e) = e {
            error!("Error: {:?}", e);
        }

        // Mark the worker as available
        self.add_available_worker(ctx.worker.take().expect("Failed to get worker"))
            .await;
    }
}
