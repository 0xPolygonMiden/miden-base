use std::time::Duration;

use pingora::lb::Backend;
use tonic::transport::Channel;
use tonic_health::pb::{
    health_check_response::ServingStatus, health_client::HealthClient, HealthCheckRequest,
};
use tracing::error;

use crate::{error::TxProverServiceError, utils::create_health_check_client};

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
}

impl Worker {
    pub async fn new(
        worker: Backend,
        connection_timeout: Duration,
        total_timeout: Duration,
    ) -> Result<Self, TxProverServiceError> {
        let health_check_client =
            create_health_check_client(worker.addr.to_string(), connection_timeout, total_timeout)
                .await?;

        Ok(Self {
            backend: worker,
            is_available: true,
            health_check_client,
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
            Ok(response) => response.into_inner().status() == ServingStatus::Serving,
            Err(err) => {
                error!("Failed to check worker health ({}): {}", self.address(), err);
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
}

impl PartialEq for Worker {
    fn eq(&self, other: &Self) -> bool {
        self.backend == other.backend
    }
}
