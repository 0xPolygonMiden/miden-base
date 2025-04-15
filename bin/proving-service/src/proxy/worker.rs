use std::time::{Duration, Instant};

use pingora::lb::Backend;
use serde::Serialize;
use tonic::transport::Channel;
use tracing::error;

use super::{metrics::WORKER_UNHEALTHY, status::create_status_client};
use crate::{
    commands::worker::ProverType,
    error::ProvingServiceError,
    generated::status::{StatusRequest, status_api_client::StatusApiClient},
};

/// The maximum exponent for the backoff.
///
/// The maximum backoff is 2^[MAX_BACKOFF_EXPONENT] seconds.
const MAX_BACKOFF_EXPONENT: usize = 9;

// WORKER
// ================================================================================================

/// A worker used for processing of requests.
///
/// A worker consists of a backend service (defined by worker address), a flag indicating wheter
/// the worker is currently available to process new requests, and a gRPC status client.
#[derive(Debug, Clone)]
pub struct Worker {
    backend: Backend,
    status_client: StatusApiClient<Channel>,
    is_available: bool,
    health_status: WorkerHealthStatus,
    version: String,
}

/// The health status of a worker.
///
/// A worker can be either healthy or unhealthy.
/// If the worker is unhealthy, it will have a number of failed attempts.
/// The number of failed attempts is incremented each time the worker is unhealthy.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum WorkerHealthStatus {
    Healthy,
    Unhealthy {
        failed_attempts: usize,
        #[serde(skip_serializing)]
        first_fail_timestamp: Instant,
        reason: String,
    },
}

impl Worker {
    /// Creates a new worker and a gRPC status client for the given worker address.
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

        let status_client =
            create_status_client(worker_addr, connection_timeout, total_timeout).await?;

        Ok(Self {
            backend,
            is_available: true,
            status_client,
            health_status: WorkerHealthStatus::Healthy,
            version: "".to_string(),
        })
    }

    /// Returns the worker address.
    pub fn address(&self) -> String {
        self.backend.addr.to_string()
    }

    /// Checks the worker status.
    ///
    /// # Returns
    /// - `Some(true)` if the worker is ready.
    /// - `Some(false)` if the worker is not ready or if there was an error checking the status.
    /// - `None` if the worker should not do a health check.
    pub async fn check_status(
        &mut self,
        supported_prover_type: &ProverType,
    ) -> Option<WorkerHealthStatus> {
        if !self.should_do_health_check() {
            return None;
        }

        let failed_attempts = self.retries_amount();

        let worker_status = match self.status_client.status(StatusRequest {}).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                error!("Failed to check worker status ({}): {}", self.address(), e);
                return Some(WorkerHealthStatus::Unhealthy {
                    failed_attempts: failed_attempts + 1,
                    first_fail_timestamp: Instant::now(),
                    reason: e.message().to_string(),
                });
            },
        };

        self.version = worker_status.version;

        let worker_supported_proof_type =
            match ProverType::try_from(worker_status.supported_proof_type) {
                Ok(proof_type) => proof_type,
                Err(e) => {
                    error!(
                        "Failed to convert worker supported proof type ({}): {}",
                        self.address(),
                        e
                    );
                    return Some(WorkerHealthStatus::Unhealthy {
                        failed_attempts: failed_attempts + 1,
                        first_fail_timestamp: Instant::now(),
                        reason: e.to_string(),
                    });
                },
            };

        if !(*supported_prover_type == worker_supported_proof_type) {
            return Some(WorkerHealthStatus::Unhealthy {
                failed_attempts: failed_attempts + 1,
                first_fail_timestamp: Instant::now(),
                reason: "Unsupported proof type".to_string(),
            });
        }

        Some(WorkerHealthStatus::Healthy)
    }

    /// Returns the worker availability.
    pub fn is_available(&self) -> bool {
        self.is_available
    }

    /// Sets the worker availability.
    pub fn set_availability(&mut self, is_available: bool) {
        self.is_available = is_available
    }

    pub(crate) fn set_health_status(&mut self, health_status: WorkerHealthStatus) {
        self.health_status = health_status;
        match &self.health_status {
            WorkerHealthStatus::Healthy => {
                self.is_available = true;
            },
            WorkerHealthStatus::Unhealthy { .. } => {
                WORKER_UNHEALTHY.with_label_values(&[&self.address()]).inc();
                self.is_available = false;
            },
        }
    }

    /// Returns the number of retries the worker has had.
    pub fn retries_amount(&self) -> usize {
        match &self.health_status {
            WorkerHealthStatus::Healthy => 0,
            WorkerHealthStatus::Unhealthy {
                failed_attempts,
                first_fail_timestamp: _,
                reason: _,
            } => *failed_attempts,
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
            WorkerHealthStatus::Unhealthy {
                failed_attempts,
                first_fail_timestamp,
                reason: _,
            } => {
                let time_since_first_failure = Instant::now() - first_fail_timestamp;
                time_since_first_failure
                    > Duration::from_secs(
                        2u64.pow(failed_attempts.min(MAX_BACKOFF_EXPONENT) as u32),
                    )
            },
        }
    }

    pub fn health_status(&self) -> &WorkerHealthStatus {
        &self.health_status
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

impl PartialEq for Worker {
    fn eq(&self, other: &Self) -> bool {
        self.backend == other.backend
    }
}
