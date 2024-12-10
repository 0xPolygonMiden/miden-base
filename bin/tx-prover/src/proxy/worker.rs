// WORKER
// ================================================================================================

use std::time::Duration;

use pingora::lb::Backend;
use tonic::transport::Channel;
use tonic_health::pb::{
    health_check_response::ServingStatus, health_client::HealthClient, HealthCheckRequest,
};
use tracing::error;

use crate::commands::WorkerConfig;

/// A worker used for processing of requests.
///
/// A worker consists of a backend service (defined by worker address), a flag indicating wheter
/// the worker is currently available to process new requests, and a gRPC health check client.
#[derive(Debug, Clone)]
pub struct Worker {
    pub(crate) worker: Backend,
    health_check_client: HealthClient<Channel>,
    pub(crate) is_available: bool,
}

impl Worker {
    pub async fn new(
        worker: Backend,
        connection_timeout: Duration,
        total_timeout: Duration,
    ) -> Self {
        let health_check_client = Self::create_health_check_client(
            worker.addr.to_string(),
            connection_timeout,
            total_timeout,
        )
        .await
        .expect("Could not create health check client");

        Self {
            worker,
            is_available: true,
            health_check_client,
        }
    }

    /// Create a gRPC [HealthClient] for the given worker address.
    ///
    /// It will panic if the worker URI is invalid.
    async fn create_health_check_client(
        address: String,
        connection_timeout: Duration,
        total_timeout: Duration,
    ) -> Result<HealthClient<Channel>, tonic::transport::Error> {
        Channel::from_shared(format!("http://{}", address))
            .expect("Invalid worker URI")
            .connect_timeout(connection_timeout)
            .timeout(total_timeout)
            .connect()
            .await
            .map(HealthClient::new)
    }

    pub fn address(&self) -> String {
        self.worker.addr.to_string()
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
}

impl PartialEq for Worker {
    fn eq(&self, other: &Self) -> bool {
        self.worker == other.worker
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
