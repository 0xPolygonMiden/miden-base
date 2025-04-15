use std::{sync::Arc, time::Duration};

use pingora::{
    apps::HttpServerApp,
    http::ResponseHeader,
    prelude::*,
    protocols::{Stream, http::ServerSession},
    server::ShutdownWatch,
};
use serde::Serialize;
use tonic::{async_trait, transport::Channel};
use tracing::error;

use super::worker::WorkerHealthStatus;
use crate::{
    commands::worker::ProverType, error::ProvingServiceError,
    generated::status::status_api_client::StatusApiClient, proxy::LoadBalancerState,
};

/// Create a gRPC [StatusApiClient] for the given worker address.
///
/// # Errors
/// - [ProvingServiceError::InvalidURI] if the worker address is invalid.
/// - [ProvingServiceError::ConnectionFailed] if the connection to the worker fails.
pub async fn create_status_client(
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

// Status of a worker
#[derive(Debug, Serialize)]
pub struct WorkerStatus {
    address: String,
    version: String,
    status: WorkerHealthStatus,
}

/// Status of the proxy
#[derive(Debug, Serialize)]
pub struct ProxyStatus {
    version: String,
    prover_type: ProverType,
    workers: Vec<WorkerStatus>,
}

/// Service that handles status requests
pub struct StatusService {
    load_balancer: Arc<LoadBalancerState>,
}

impl StatusService {
    pub fn new(load_balancer: Arc<LoadBalancerState>) -> Self {
        Self { load_balancer }
    }

    async fn handle_request(&self, _session: &mut ServerSession) -> Result<()> {
        let workers = self.load_balancer.workers.read().await;
        let worker_statuses: Vec<WorkerStatus> = workers
            .iter()
            .map(|w| WorkerStatus {
                address: w.address(),
                version: w.version().to_string(),
                status: w.health_status().clone(),
            })
            .collect();

        let status = ProxyStatus {
            version: env!("CARGO_PKG_VERSION").to_string(),
            prover_type: self.load_balancer.supported_prover_type,
            workers: worker_statuses,
        };

        let response = serde_json::to_string(&status).map_err(|e| {
            Error::explain(
                ErrorType::Custom("Failed to serialize status"),
                format!("Failed to serialize status: {}", e),
            )
        })?;

        let mut header = ResponseHeader::build(200, None)?;
        header.insert_header("Content-Type", "application/json")?;
        _session.write_response_header(Box::new(header)).await?;
        _session.write_response_body(response.into(), true).await?;

        Ok(())
    }
}

#[async_trait]
impl HttpServerApp for StatusService {
    async fn process_new_http(
        self: &Arc<Self>,
        mut session: ServerSession,
        _shutdown: &ShutdownWatch,
    ) -> Option<Stream> {
        match session.read_request().await {
            Ok(false) => return None,
            Err(e) => {
                error!("Failed to read request: {}", e);
                return None;
            },
            Ok(true) => {},
        }

        if session.req_header().uri.path() != "/status" {
            return None;
        }

        let _ = self.handle_request(&mut session).await.map_err(|e| {
            error!("Failed to handle status request: {}", e);
            None::<Stream>
        });
        None
    }
}
