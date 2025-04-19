use std::time::{Duration, Instant};

use pingora::lb::Backend;
use tonic::transport::Channel;
use tonic_health::pb::{
    HealthCheckRequest, health_check_response::ServingStatus, health_client::HealthClient,
};
use tracing::error;

use super::health_check::create_health_check_client;
use crate::error::ProvingServiceError;

/// The maximum exponent for the backoff.
///
/// The maximum backoff is 2^[MAX_BACKOFF_EXPONENT] seconds.
const MAX_BACKOFF_EXPONENT: usize = 9;

// WORKER
// ================================================================================================

/// A worker used for processing of requests.
///
/// A worker consists of a backend service (defined by worker address), a flag indicating whether
/// the worker is currently available to process new requests, and a gRPC health check client.
#[derive(Debug, Clone)]
pub struct Worker {
    backend: Backend,
    health_check_client: HealthClient<Channel>,
    is_available: bool,
    health_status: WorkerHealthStatus,
}

/// The health status of a worker.
///
/// A worker can be either healthy or unhealthy.
/// If the worker is unhealthy, it will have a number of failed attempts.
/// The number of failed attempts is incremented each time the worker is unhealthy.
#[derive(Debug, Clone, PartialEq)]
pub enum WorkerHealthStatus {
    Healthy,
    Unhealthy {
        failed_attempts: usize,
        first_fail_timestamp: Instant,
    },
}

impl Worker {
    /// Creates a new worker and a gRPC health check client for the given worker address.
    ///
    /// # Errors
    /// - Returns [ProvingServiceError::InvalidURI] if the worker address is invalid.
    /// - Returns [ProvingServiceError::ConnectionFailed] if the connection to the worker fails.
    pub async fn new(
        worker_addr: String,
        connection_timeout: Duration,
        total_timeout: Duration,
    ) -> Result<Self, ProvingServiceError> {
        let backend =
            Backend::new(&worker_addr).map_err(ProvingServiceError::BackendCreationFailed)?;

        let health_check_client =
            create_health_check_client(worker_addr, connection_timeout, total_timeout).await?;

        Ok(Self {
            backend,
            is_available: true,
            health_check_client,
            health_status: WorkerHealthStatus::Healthy,
        })
    }

    /// Returns the worker address.
    pub fn address(&self) -> String {
        self.backend.addr.to_string()
    }

    /// Checks the worker health.
    ///
    /// # Returns
    /// - `Some(true)` if the worker is healthy.
    /// - `Some(false)` if the worker is unhealthy.
    /// - `None` if the worker should not do a health check.
    pub async fn is_healthy(&mut self) -> Option<bool> {
        if !self.should_do_health_check() {
            return None;
        }

        Some(
            self.health_check_client
                .check(HealthCheckRequest { service: "".to_string() })
                .await
                .map(|response| response.into_inner().status() == ServingStatus::Serving)
                .unwrap_or_else(|err| {
                    error!("Failed to check worker health ({}): {}", self.address(), err);
                    false
                }),
        )
    }

    /// Returns the worker availability.
    pub fn is_available(&self) -> bool {
        self.is_available
    }

    /// Sets the worker availability.
    pub fn set_availability(&mut self, is_available: bool) {
        self.is_available = is_available;
    }

    /// Marks the worker as unhealthy and increments the number of retries.
    ///
    /// Additionally, the worker is set to unavailable.
    pub fn mark_as_unhealthy(&mut self) {
        self.health_status = match &self.health_status {
            WorkerHealthStatus::Healthy => WorkerHealthStatus::Unhealthy {
                failed_attempts: 1,
                first_fail_timestamp: Instant::now(),
            },
            WorkerHealthStatus::Unhealthy { failed_attempts, first_fail_timestamp } => {
                WorkerHealthStatus::Unhealthy {
                    failed_attempts: failed_attempts + 1,
                    first_fail_timestamp: *first_fail_timestamp,
                }
            },
        };
        self.is_available = false;
    }

    /// Resets the health status to healthy and sets the worker to available.
    pub fn mark_as_healthy(&mut self) {
        self.health_status = WorkerHealthStatus::Healthy;
        self.is_available = true;
    }

    /// Returns the number of retries the worker has had.
    pub fn retries_amount(&self) -> usize {
        match &self.health_status {
            WorkerHealthStatus::Healthy => 0,
            WorkerHealthStatus::Unhealthy { failed_attempts, first_fail_timestamp: _ } => {
                *failed_attempts
            },
        }
    }

    /// Returns whether the worker should do a health check.
    ///
    /// A worker should do a health check if it is healthy or if the time since the first failure
    /// is greater than the time since the first failure power of 2.
    ///
    /// The maximum exponent is [MAX_BACKOFF_EXPONENT], which corresponds to a backoff of
    /// 2^[MAX_BACKOFF_EXPONENT] seconds.
    pub(crate) fn should_do_health_check(&self) -> bool {
        match self.health_status {
            WorkerHealthStatus::Healthy => true,
            WorkerHealthStatus::Unhealthy { failed_attempts, first_fail_timestamp } => {
                let time_since_first_failure = Instant::now() - first_fail_timestamp;
                time_since_first_failure
                    > Duration::from_secs(
                        2u64.pow(failed_attempts.min(MAX_BACKOFF_EXPONENT) as u32),
                    )
            },
        }
    }
}

impl PartialEq for Worker {
    fn eq(&self, other: &Self) -> bool {
        self.backend == other.backend
    }
}
