use std::time::Duration;

use pingora::lb::Backend;
use tonic::transport::Channel;
use tonic_health::pb::{
    HealthCheckRequest, health_check_response::ServingStatus, health_client::HealthClient,
};
use tracing::error;

use super::health_check::create_health_check_client;
use crate::error::ProvingServiceError;

// WORKER
// ================================================================================================

/// A worker used for processing of requests.
///
/// A worker consists of a backend service (defined by worker address), a flag indicating wheter
/// the worker is currently available to process new requests, and a gRPC health check client.
#[derive(Debug, Clone)]
pub struct Worker {
    backend: Backend,
    health_check_client: HealthClient<Channel>,
    is_available: bool,
    health_status: WorkerHealthStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkerHealthStatus {
    Healthy,
    Unhealthy { failed_attempts: usize },
}

impl Worker {
    /// Creates a new worker and a gRPC health check client for the given worker address.
    ///
    /// # Errors
    /// - Returns [ProvingServiceError::InvalidURI] if the worker address is invalid.
    /// - Returns [ProvingServiceError::ConnectionFailed] if the connection to the worker fails.
    pub async fn new(
        worker: Backend,
        connection_timeout: Duration,
        total_timeout: Duration,
    ) -> Result<Self, ProvingServiceError> {
        let health_check_client =
            create_health_check_client(worker.addr.to_string(), connection_timeout, total_timeout)
                .await?;

        Ok(Self {
            backend: worker,
            is_available: true,
            health_check_client,
            health_status: WorkerHealthStatus::Healthy,
        })
    }

    pub fn address(&self) -> String {
        self.backend.addr.to_string()
    }

    pub async fn is_healthy(&mut self) -> bool {
        match self
            .health_check_client
            .check(HealthCheckRequest { service: "".to_string() })
            .await
        {
            Ok(response) => {
                let status = response.into_inner().status();
                if status == ServingStatus::Serving {
                    self.mark_as_healthy();
                } else {
                    self.mark_as_unhealthy();
                }
                status == ServingStatus::Serving
            },
            Err(err) => {
                error!("Failed to check worker health ({}): {}", self.address(), err);
                self.mark_as_unhealthy();
                false
            },
        }
    }

    pub fn is_available(&self) -> bool {
        self.is_available
    }

    pub fn set_availability(&mut self, is_available: bool) {
        self.is_available = is_available;
    }

    pub fn mark_as_unhealthy(&mut self) {
        self.health_status = match &self.health_status {
            WorkerHealthStatus::Healthy => WorkerHealthStatus::Unhealthy { failed_attempts: 1 },
            WorkerHealthStatus::Unhealthy { failed_attempts } => {
                WorkerHealthStatus::Unhealthy { failed_attempts: failed_attempts + 1 }
            },
        };
        self.is_available = false;
    }

    /// Reset the health status to healthy
    pub fn mark_as_healthy(&mut self) {
        if self.health_status == WorkerHealthStatus::Healthy {
            return;
        }

        self.health_status = WorkerHealthStatus::Healthy;
        self.is_available = true;
    }

    pub fn has_exceeded_failed_health_checks(&self, max_health_check_retries: usize) -> bool {
        match &self.health_status {
            WorkerHealthStatus::Healthy => false,
            WorkerHealthStatus::Unhealthy { failed_attempts } => {
                failed_attempts >= &max_health_check_retries
            },
        }
    }
}

impl PartialEq for Worker {
    fn eq(&self, other: &Self) -> bool {
        self.backend == other.backend
    }
}
