use std::time::Duration;

use tonic::transport::Channel;

use crate::{error::ProvingServiceError, generated::status::status_api_client::StatusApiClient};

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
