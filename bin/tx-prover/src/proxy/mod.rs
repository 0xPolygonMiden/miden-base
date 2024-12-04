use std::{collections::VecDeque, future::Future, pin::Pin, sync::Arc, time::Duration};

use async_trait::async_trait;
use once_cell::sync::Lazy;
use pingora::{
    lb::Backend,
    prelude::*,
    server::ShutdownWatch,
    services::background::BackgroundService,
    upstreams::peer::{Peer, ALPN},
};
use pingora_core::{upstreams::peer::HttpPeer, Result};
use pingora_limits::rate::Rate;
use pingora_proxy::{ProxyHttp, Session};
use tokio::{sync::RwLock, time::sleep};
use tonic::transport::Channel;
use tonic_health::pb::{
    health_check_response::ServingStatus, health_client::HealthClient, HealthCheckRequest,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    commands::{
        update_workers::{Action, UpdateWorkers},
        ProxyConfig, WorkerConfig,
    },
    utils::{
        create_queue_full_response, create_response_with_error_message,
        create_too_many_requests_response, create_workers_updated_response,
    },
};

/// Localhost address
const LOCALHOST_ADDR: &str = "127.0.0.1";

// WORKER
// ================================================================================================

/// Worker
///
/// A worker is a backend server that processes requests. It is represented by its address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worker {
    worker: Backend,
    is_available: bool,
}

impl Worker {
    pub fn new(worker: Backend) -> Self {
        Self { worker, is_available: true }
    }

    /// Update the worker configuration in the configuration file.
    ///
    /// # Errors
    /// - If the worker address cannot be converted to [WorkerConfig].
    fn update_worker_config(workers: &[Worker]) -> Result<(), String> {
        let worker_configs =
            workers.iter().map(|worker| worker.try_into()).collect::<Result<Vec<_>, _>>()?;

        ProxyConfig::update_workers(worker_configs)
    }
}

impl TryInto<WorkerConfig> for &Worker {
    type Error = String;

    fn try_into(self) -> std::result::Result<WorkerConfig, String> {
        self.worker
            .as_inet()
            .ok_or_else(|| "Failed to get worker address".to_string())
            .map(|worker_addr| WorkerConfig::new(&worker_addr.ip().to_string(), worker_addr.port()))
    }
}

// LOAD BALANCER
// ================================================================================================

/// Load balancer that uses a round robin strategy
pub struct LoadBalancer {
    workers: Arc<RwLock<Vec<Worker>>>,
    timeout_secs: Duration,
    connection_timeout_secs: Duration,
    max_queue_items: usize,
    max_retries_per_request: usize,
    max_req_per_sec: isize,
    available_workers_polling_time: Duration,
    health_check_frequency: Duration,
}

impl LoadBalancer {
    /// Create a new load balancer
    pub fn new(workers: Vec<Backend>, config: &ProxyConfig) -> Self {
        let workers: Vec<Worker> = workers.into_iter().map(Worker::new).collect();

        Self {
            workers: Arc::new(RwLock::new(workers.clone())),
            timeout_secs: Duration::from_secs(config.timeout_secs),
            connection_timeout_secs: Duration::from_secs(config.connection_timeout_secs),
            max_queue_items: config.max_queue_items,
            max_retries_per_request: config.max_retries_per_request,
            max_req_per_sec: config.max_req_per_sec,
            available_workers_polling_time: Duration::from_millis(
                config.available_workers_polling_time_ms,
            ),
            health_check_frequency: Duration::from_secs(config.health_check_interval_secs),
        }
    }

    /// Gets an available worker and marks it as unavailable.
    ///
    /// If no worker is available, it will return None.
    pub async fn pop_available_worker(&self) -> Option<Worker> {
        let mut available_workers = self.workers.write().await;
        available_workers.iter_mut().find(|w| w.is_available).map(|w| {
            w.is_available = false;
            w.clone()
        })
    }

    /// Marks the given worker as available.
    ///
    /// If the worker is not in the list, it won't be added.
    pub async fn add_available_worker(&self, worker: Backend) {
        let mut available_workers = self.workers.write().await;
        if let Some(w) = available_workers.iter_mut().find(|w| w.worker == worker) {
            w.is_available = true;
        }
    }

    /// Updates the list of available workers based on the given action ("add" or "remove").
    ///
    /// # Behavior
    ///
    /// ## Add Action
    /// - If the worker exists in the current workers list, do nothing.
    /// - Otherwise, add it and mark it as available.
    ///
    /// ## Remove Action
    /// - If the worker exists in the current workers list, remove it.
    /// - Otherwise, do nothing.
    ///
    /// Finally, updates the configuration file with the new list of workers.
    ///
    /// # Errors
    /// - If the worker cannot be created.
    /// - If the configuration cannot be loaded.
    /// - If the configuration cannot be saved.
    pub async fn update_workers(
        &self,
        update_workers: UpdateWorkers,
    ) -> std::result::Result<(), String> {
        let mut workers = self.workers.write().await;

        info!("Current workers: {:?}", workers);

        let workers_to_update: Vec<Worker> = update_workers
            .workers
            .iter()
            .map(|worker| Backend::new(worker))
            .collect::<Result<Vec<Backend>, _>>()
            .map_err(|err| format!("Failed to create backend: {}", err))?
            .into_iter()
            .map(Worker::new)
            .collect();

        match update_workers.action {
            Action::Add => {
                for worker in workers_to_update {
                    if !workers.iter().any(|w| w.worker == worker.worker) {
                        workers.push(worker);
                    }
                }
            },
            Action::Remove => {
                for worker in workers_to_update {
                    workers.retain(|w| w.worker != worker.worker);
                }
            },
        }

        let new_list_of_workers =
            workers.iter().map(|worker| worker.try_into()).collect::<Result<Vec<_>, _>>()?;

        ProxyConfig::update_workers(new_list_of_workers)?;

        info!("Workers updated: {:?}", workers);

        Ok(())
    }

    /// Get the total number of current workers.
    pub async fn num_workers(&self) -> usize {
        self.workers.read().await.len()
    }

    /// Handles the update workers request.
    ///
    /// # Behavior
    /// - Reads the HTTP request from the session.
    /// - If query parameters are present, attempts to parse them as an `UpdateWorkers` object.
    /// - If the parsing fails, returns an error response.
    /// - If successful, updates the list of workers by calling `update_workers`.
    /// - If the update is successful, returns the count of available workers.
    ///
    /// # Errors
    /// - If the HTTP request cannot be read.
    /// - If the query parameters cannot be parsed.
    /// - If the workers cannot be updated.
    /// - If the response cannot be created.
    pub async fn handle_update_workers_request(
        &self,
        session: &mut Session,
    ) -> Option<Result<bool>> {
        let http_session = session.as_downstream_mut();

        // Attempt to read the HTTP request
        if let Err(err) = http_session.read_request().await {
            let error_message = format!("Failed to read request: {}", err);
            error!("{}", error_message);
            return Some(create_response_with_error_message(session, error_message).await);
        }

        // Extract and parse query parameters, if there are not any, return early to continue
        // processing the request as a regular proving request.
        let query_params = match http_session.req_header().as_ref().uri.query() {
            Some(params) => params,
            None => {
                return None;
            },
        };

        // Parse the query parameters
        let update_workers: Result<UpdateWorkers, _> = serde_qs::from_str(query_params);
        let update_workers = match update_workers {
            Ok(workers) => workers,
            Err(err) => {
                let error_message = format!("Failed to parse query parameters: {}", err);
                error!("{}", error_message);
                return Some(create_response_with_error_message(session, error_message).await);
            },
        };

        // Update workers and handle potential errors
        if let Err(err) = self.update_workers(update_workers).await {
            let error_message = format!("Failed to update workers: {}", err);
            error!("{}", error_message);
            return Some(create_response_with_error_message(session, error_message).await);
        }

        // Successfully updated workers
        info!("Workers updated successfully");
        let workers_count = self.num_workers().await;
        Some(create_workers_updated_response(session, workers_count).await)
    }

    /// Create a gRPC client for the given worker address.
    ///
    /// It will panic if the worker URI is invalid.
    async fn create_grpc_client(
        &self,
        worker_addr: &str,
    ) -> Result<HealthClient<Channel>, tonic::transport::Error> {
        Channel::from_shared(format!("http://{}", worker_addr))
            .expect("Invalid worker URI")
            .connect_timeout(self.connection_timeout_secs)
            .timeout(self.timeout_secs)
            .connect()
            .await
            .map(HealthClient::new)
    }

    /// Check the health of the workers and returns a list of healthy workers.
    ///
    /// Performs a health check on each worker using the gRPC health check protocol. If a worker
    /// is not healthy, it won't be included in the list of healthy workers.
    async fn check_workers_health(
        &self,
        workers: impl Iterator<Item = &mut Worker>,
    ) -> Vec<Worker> {
        let mut healthy_workers = Vec::new();

        for worker in workers {
            let worker_addr = worker.worker.addr.clone();
            match self.create_grpc_client(&worker_addr.to_string()).await {
                Ok(mut client) => {
                    match client.check(HealthCheckRequest { service: "".to_string() }).await {
                        Ok(response) => {
                            if response.into_inner().status() == ServingStatus::Serving {
                                debug!("Worker {} is healthy", worker_addr);
                                healthy_workers.push(worker.clone());
                            } else {
                                warn!("Worker {} is not healthy", worker_addr);
                            }
                        },
                        Err(err) => {
                            error!("Failed to check worker health ({}): {}", worker_addr, err);
                        },
                    }
                },
                Err(err) => {
                    error!("Failed to connect to worker {}: {}", worker_addr, err);
                },
            }
        }

        healthy_workers
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

// LOAD BALANCER WRAPPER
// ================================================================================================

/// Wrapper around the load balancer that implements the ProxyHttp trait
///
/// This wrapper is used to implement the ProxyHttp trait for Arc<LoadBalancer>.
/// This is necessary because we want to share the load balancer between the proxy server and the
/// health check background service.
pub struct LoadBalancerWrapper(pub Arc<LoadBalancer>);

/// Implements load-balancing of incoming requests across a pool of workers.
///
/// At the backend-level, a request lifecycle works as follows:
/// - When a new requests arrives, [LoadBalancer::request_filter()] method is called. In this method
///   we apply IP-based rate-limiting to the request and check if the request queue is full. In this
///   method we also handle the special case update workers request.
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
impl ProxyHttp for LoadBalancerWrapper {
    type CTX = RequestContext;
    fn new_ctx(&self) -> Self::CTX {
        RequestContext::new()
    }

    /// Decide whether to filter the request or not. Also, handle the special case of the update
    /// workers request.
    ///
    /// Here we apply IP-based rate-limiting to the request. We also check if the queue is full.
    ///
    /// If the request is rate-limited, we return a 429 response. Otherwise, we return false.
    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        // Extract the client address early
        let client_addr = match session.client_addr() {
            Some(addr) => addr.to_string(),
            None => {
                return create_response_with_error_message(
                    session,
                    "No socket address".to_string(),
                )
                .await;
            },
        };

        info!("Client address: {:?}", client_addr);

        // Special handling for localhost
        if client_addr.contains(LOCALHOST_ADDR) {
            if let Some(response) = self.0.handle_update_workers_request(session).await {
                return response;
            }
        }

        let user_id = Some(client_addr);

        // Retrieve the current window requests
        let curr_window_requests = RATE_LIMITER.observe(&user_id, 1);

        // Rate limit the request
        if curr_window_requests > self.0.max_req_per_sec {
            return create_too_many_requests_response(session, self.0.max_req_per_sec).await;
        };

        let queue_len = QUEUE.len().await;

        info!("New request with ID: {}", ctx.request_id);
        info!("Queue length: {}", queue_len);

        // Check if the queue is full
        if queue_len >= self.0.max_queue_items {
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
            if let Some(worker) = self.0.pop_available_worker().await {
                info!(
                    "Worker {} picked up the request with ID: {}",
                    worker.worker.addr, request_id
                );
                ctx.set_worker(worker.worker);
                break;
            }
            info!("All workers are busy");
            tokio::time::sleep(self.0.available_workers_polling_time).await;
        }

        // Remove the request from the queue
        QUEUE.dequeue().await;

        // Set SNI
        let mut http_peer =
            HttpPeer::new(ctx.worker.clone().expect("Failed to get worker"), false, "".to_string());
        let peer_opts =
            http_peer.get_mut_peer_options().ok_or(Error::new(ErrorType::InternalError))?;

        // Timeout settings
        peer_opts.total_connection_timeout = Some(self.0.timeout_secs);
        peer_opts.connection_timeout = Some(self.0.connection_timeout_secs);
        peer_opts.read_timeout = Some(self.0.timeout_secs);
        peer_opts.write_timeout = Some(self.0.timeout_secs);
        peer_opts.idle_timeout = Some(self.0.timeout_secs);

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
        if ctx.tries > self.0.max_retries_per_request {
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
        if let Some(worker) = ctx.worker.take() {
            self.0.add_available_worker(worker).await;
        }
    }
}

/// Implement the BackgroundService trait for the LoadBalancer
///
/// A [BackgroundService] can be run as part of a Pingora application to add supporting logic that
/// exists outside of the request/response lifecycle.
///
/// We use this implementation to periodically check the health of the workers and update the list
/// of available workers.
impl BackgroundService for LoadBalancer {
    /// Starts the health check background service.
    ///
    /// This function is called when the Pingora server tries to start all the services. The
    /// background service can return at anytime or wait for the `shutdown` signal.
    ///
    /// The health check background service will periodically check the health of the workers
    /// using the gRPC health check protocol. If a worker is not healthy, it will be removed from
    /// the list of available workers.
    ///
    /// # Errors
    /// - If the worker has an invalid URI.
    /// - If a [WorkerConfig] cannot be created from a given [Worker].
    fn start<'life0, 'async_trait>(
        &'life0 self,
        _shutdown: ShutdownWatch,
    ) -> Pin<Box<dyn Future<Output = ()> + ::core::marker::Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            loop {
                let mut workers = self.workers.write().await;

                // Perform health checks on workers and retain healthy ones
                let healthy_workers = self.check_workers_health(workers.iter_mut()).await;

                // Update the worker list with healthy workers
                *workers = healthy_workers;

                // Persist the updated worker list to the configuration file
                if let Err(err) = Worker::update_worker_config(&workers) {
                    error!("Failed to update workers in the configuration file: {}", err);
                }

                // Sleep for the defined interval before the next health check
                sleep(self.health_check_frequency).await;
            }
        })
    }
}
