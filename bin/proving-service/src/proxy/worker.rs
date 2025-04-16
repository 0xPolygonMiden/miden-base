use std::time::{Duration, Instant};

use pingora::lb::Backend;
use serde::Serialize;
use tonic::transport::Channel;
use tracing::error;

use super::metrics::WORKER_UNHEALTHY;
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
    /// The worker is healthy.
    Healthy,
    /// The worker is unhealthy.
    Unhealthy {
        /// The number of failed attempts.
        num_failed_attempts: usize,
        /// The timestamp of the first failure.
        #[serde(skip_serializing)]
        first_fail_timestamp: Instant,
        /// The reason for the failure.
        reason: String,
    },
    /// The worker status is unknown.
    Unknown,
}

impl Worker {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

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
            health_status: WorkerHealthStatus::Unknown,
            version: "".to_string(),
        })
    }

    // MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Checks the worker status.
    ///
    /// # Returns
    /// - `Some(true)` if the worker is ready.
    /// - `Some(false)` if the worker is not ready or if there was an error checking the status.
    /// - `None` if the worker should not do a health check.
    pub async fn check_status(&mut self, supported_prover_type: ProverType) {
        if !self.should_do_health_check() {
            return;
        }

        let failed_attempts = self.num_failures();

        let worker_status = match self.status_client.status(StatusRequest {}).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                error!("Failed to check worker status ({}): {}", self.address(), e);
                self.set_health_status(WorkerHealthStatus::Unhealthy {
                    num_failed_attempts: failed_attempts + 1,
                    first_fail_timestamp: Instant::now(),
                    reason: e.message().to_string(),
                });
                return;
            },
        };

        if worker_status.version.is_empty() {
            self.set_health_status(WorkerHealthStatus::Unhealthy {
                num_failed_attempts: failed_attempts + 1,
                first_fail_timestamp: Instant::now(),
                reason: "Worker version is empty".to_string(),
            });
            return;
        }

        if !self.is_valid_version(&worker_status.version) {
            self.set_health_status(WorkerHealthStatus::Unhealthy {
                num_failed_attempts: failed_attempts + 1,
                first_fail_timestamp: Instant::now(),
                reason: "Worker version is invalid".to_string(),
            });
            return;
        }

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
                    self.set_health_status(WorkerHealthStatus::Unhealthy {
                        num_failed_attempts: failed_attempts + 1,
                        first_fail_timestamp: Instant::now(),
                        reason: e.to_string(),
                    });
                    return;
                },
            };

        if supported_prover_type != worker_supported_proof_type {
            self.set_health_status(WorkerHealthStatus::Unhealthy {
                num_failed_attempts: failed_attempts + 1,
                first_fail_timestamp: Instant::now(),
                reason: format!("Unsupported prover type: {}", worker_supported_proof_type),
            });
            return;
        }

        self.set_health_status(WorkerHealthStatus::Healthy);
    }

    /// Sets the worker availability.
    pub fn set_availability(&mut self, is_available: bool) {
        self.is_available = is_available
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the number of failures the worker has had.
    pub fn num_failures(&self) -> usize {
        match &self.health_status {
            WorkerHealthStatus::Healthy => 0,
            WorkerHealthStatus::Unhealthy {
                num_failed_attempts: failed_attempts,
                first_fail_timestamp: _,
                reason: _,
            } => *failed_attempts,
            WorkerHealthStatus::Unknown => 0,
        }
    }

    /// Returns the health status of the worker.
    pub fn health_status(&self) -> &WorkerHealthStatus {
        &self.health_status
    }

    /// Returns the version of the worker.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the worker availability.
    pub fn is_available(&self) -> bool {
        self.is_available
    }

    /// Returns the worker address.
    pub fn address(&self) -> String {
        self.backend.addr.to_string()
    }

    // PRIVATE HELPERS
    // --------------------------------------------------------------------------------------------

    /// Returns whether the worker should do a health check.
    ///
    /// A worker should do a health check if it is healthy or if the time since the first failure
    /// is greater than the time since the first failure power of 2.
    ///
    /// The maximum exponent is [MAX_BACKOFF_EXPONENT], which corresponds to a backoff of
    /// 2^[MAX_BACKOFF_EXPONENT] seconds.
    fn should_do_health_check(&self) -> bool {
        match self.health_status {
            WorkerHealthStatus::Healthy => true,
            WorkerHealthStatus::Unhealthy {
                num_failed_attempts: failed_attempts,
                first_fail_timestamp,
                reason: _,
            } => {
                let time_since_first_failure = Instant::now() - first_fail_timestamp;
                time_since_first_failure
                    > Duration::from_secs(
                        2u64.pow(failed_attempts.min(MAX_BACKOFF_EXPONENT) as u32),
                    )
            },
            WorkerHealthStatus::Unknown => true,
        }
    }

    /// Sets the health status of the worker.
    fn set_health_status(&mut self, health_status: WorkerHealthStatus) {
        let previous_health_status = self.health_status.clone();
        self.health_status = health_status;
        match &self.health_status {
            WorkerHealthStatus::Healthy => {
                if matches!(previous_health_status, WorkerHealthStatus::Unhealthy { .. }) {
                    self.is_available = true;
                }
            },
            WorkerHealthStatus::Unhealthy { .. } => {
                WORKER_UNHEALTHY.with_label_values(&[&self.address()]).inc();
                self.is_available = false;
            },
            WorkerHealthStatus::Unknown => {
                if matches!(previous_health_status, WorkerHealthStatus::Unhealthy { .. }) {
                    self.is_available = true;
                }
            },
        }
    }

    /// Returns whether the worker version is valid.
    ///
    /// The version is valid if it is a semantic version and is greater than or equal to the
    /// current version. We dont check the patch version.
    fn is_valid_version(&self, version: &str) -> bool {
        // Dont check the patch version.
        let current_version = env!("CARGO_PKG_VERSION");
        let current_version_parts: Vec<&str> = current_version.split('.').collect();
        let version_parts: Vec<&str> = version.split('.').collect();
        version_parts[0] == current_version_parts[0] && version_parts[1] == current_version_parts[1]
    }
}

// PARTIAL EQUALITY
// ================================================================================================

impl PartialEq for Worker {
    fn eq(&self, other: &Self) -> bool {
        self.backend == other.backend
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Create a gRPC [StatusApiClient] for the given worker address.
///
/// # Errors
/// - [ProvingServiceError::InvalidURI] if the worker address is invalid.
/// - [ProvingServiceError::ConnectionFailed] if the connection to the worker fails.
async fn create_status_client(
    address: String,
    connection_timeout: Duration,
    total_timeout: Duration,
) -> Result<StatusApiClient<Channel>, ProvingServiceError> {
    let channel = Channel::from_shared(format!("http://{}", address))
        .map_err(|err| ProvingServiceError::InvalidURI(err, address.clone()))?
        .connect_timeout(connection_timeout)
        .timeout(total_timeout)
        .connect()
        .await
        .map_err(|err| ProvingServiceError::ConnectionFailed(err, address))?;

    Ok(StatusApiClient::new(channel))
}
