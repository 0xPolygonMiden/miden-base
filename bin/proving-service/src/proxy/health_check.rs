use std::time::Duration;

use pingora::{prelude::sleep, server::ShutdownWatch, services::background::BackgroundService};
use tonic::{async_trait, transport::Channel};
use tonic_health::pb::health_client::HealthClient;
use tracing::{debug_span, info};

use super::{
    LoadBalancerState,
    metrics::{WORKER_COUNT, WORKER_UNHEALTHY},
};
use crate::error::ProvingServiceError;

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
    async fn start(&self, mut _shutdown: ShutdownWatch) {
        Box::pin(async move {
            loop {
                info!("Starting health check");
                // Create a new spawn to perform the health check
                let span = debug_span!("proxy:health_check");
                let _guard = span.enter();
                {
                    let mut workers = self.workers.write().await;

                    info!("workers: {:?}", workers);

                    // Perform health checks on workers and retain healthy ones
                    self.check_workers_health(workers.iter_mut()).await;

                    // Count workers before filtering
                    let before_count = workers.len();

                    info!("max health check retries: {}", self.max_health_check_retries);

                    // Filter out unhealthy workers that exceed retry limit
                    workers.retain(|worker| {
                        !worker.has_exceeded_failed_health_checks(self.max_health_check_retries)
                    });

                    // Log and update metrics for removed workers
                    let removed_count = before_count - workers.len();
                    if removed_count > 0 {
                        WORKER_UNHEALTHY.inc_by(removed_count as u64);
                        info!(
                            "Removed {} workers that exceeded health check retry limit",
                            removed_count
                        );
                    }

                    // Update total worker count
                    WORKER_COUNT.set(workers.len() as i64);

                    // Sleep for the defined interval before the next health check
                }
                sleep(self.health_check_interval).await;
            }
        })
        .await;
    }
}

// HELPERS
// ================================================================================================

/// Create a gRPC [HealthClient] for the given worker address.
///
/// # Errors
/// - [ProvingServiceError::InvalidURI] if the worker address is invalid.
/// - [ProvingServiceError::ConnectionFailed] if the connection to the worker fails.
pub async fn create_health_check_client(
    address: String,
    connection_timeout: Duration,
    total_timeout: Duration,
) -> Result<HealthClient<Channel>, ProvingServiceError> {
    let channel = Channel::from_shared(format!("http://{}", address))
        .map_err(|err| ProvingServiceError::InvalidURI(err, address.clone()))?
        .connect_timeout(connection_timeout)
        .timeout(total_timeout)
        .connect()
        .await
        .map_err(|err| ProvingServiceError::ConnectionFailed(err, address))?;

    Ok(HealthClient::new(channel))
}
