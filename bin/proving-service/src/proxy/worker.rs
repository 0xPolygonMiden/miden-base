use std::time::Duration;

use pingora::lb::Backend;
use tonic::transport::Channel;
use tracing::error;

use super::status::create_status_client;
use crate::{error::ProvingServiceError, generated::status::status_api_client::StatusApiClient};

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
}

impl Worker {
    /// Creates a new worker and a gRPC status client for the given worker address.
    ///
    /// # Errors
    /// - Returns [ProvingServiceError::InvalidURI] if the worker address is invalid.
    /// - Returns [ProvingServiceError::ConnectionFailed] if the connection to the worker fails.
    pub async fn new(
        worker: Backend,
        connection_timeout: Duration,
        total_timeout: Duration,
    ) -> Result<Self, ProvingServiceError> {
        let status_client =
            create_status_client(worker.addr.to_string(), connection_timeout, total_timeout)
                .await?;

        Ok(Self {
            backend: worker,
            is_available: true,
            status_client,
        })
    }

    pub fn address(&self) -> String {
        self.backend.addr.to_string()
    }

    pub async fn is_ready(&mut self) -> bool {
        match self.status_client.status(()).await {
            Ok(response) => response.into_inner().ready,
            Err(err) => {
                error!("Failed to check worker status ({}): {}", self.address(), err);
                false
            },
        }
    }

    pub fn is_available(&self) -> bool {
        self.is_available
    }

    pub fn set_availability(&mut self, is_available: bool) {
        self.is_available = is_available
    }
}

impl PartialEq for Worker {
    fn eq(&self, other: &Self) -> bool {
        self.backend == other.backend
    }
}
