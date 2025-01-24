use axum::async_trait;
use pingora::{prelude::sleep, server::ShutdownWatch, services::background::BackgroundService};
use tracing::debug_span;

use super::{
    metrics::{WORKER_COUNT, WORKER_UNHEALTHY},
    LoadBalancerState,
};

/// Implement the BackgroundService trait for the LoadBalancer
///
/// A [BackgroundService] can be run as part of a Pingora application to add supporting logic that
/// exists outside of the request/response lifecycle.
///
/// We use this implementation to periodically check the health of the workers and update the list
/// of available workers.
#[async_trait]
impl BackgroundService for LoadBalancerState {
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
    async fn start(&self, _shutdown: ShutdownWatch) {
        Box::pin(async move {
            loop {
                // Create a new spawn to perform the health check
                let span = debug_span!("proxy:health_check");
                let _guard = span.enter();

                let mut workers = self.workers.write().await;
                let initial_workers_len = workers.len();

                // Perform health checks on workers and retain healthy ones
                let healthy_workers = self.check_workers_health(workers.iter_mut()).await;

                // Update the worker list with healthy workers
                *workers = healthy_workers;

                // Update the worker count and worker unhealhy count metrics
                WORKER_COUNT.set(workers.len() as i64);
                let unhealthy_workers = initial_workers_len - workers.len();
                WORKER_UNHEALTHY.inc_by(unhealthy_workers as u64);

                // Sleep for the defined interval before the next health check
                sleep(self.health_check_frequency).await;
            }
        })
        .await;
    }
}
