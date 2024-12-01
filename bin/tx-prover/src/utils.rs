use figment::{
    providers::{Format, Toml},
    Figment,
};
use miden_tx_prover::PROVER_SERVICE_CONFIG_FILE_NAME;
use pingora::{http::ResponseHeader, Error, ErrorType};
use pingora_proxy::Session;

use crate::commands::ProxyConfig;

const RESOURCE_EXHAUSTED_CODE: u16 = 8;

/// Loads config file from current directory and default filename and returns it
///
/// This function will look for the configuration file with the name defined at the
/// [PROVER_SERVICE_CONFIG_FILE_NAME] constant in the current directory.
pub(crate) fn load_config_from_file() -> Result<ProxyConfig, String> {
    let mut current_dir = std::env::current_dir().map_err(|err| err.to_string())?;
    current_dir.push(PROVER_SERVICE_CONFIG_FILE_NAME);
    let config_path = current_dir.as_path();

    Figment::from(Toml::file(config_path))
        .extract()
        .map_err(|err| format!("Failed to load {} config file: {err}", config_path.display()))
}

pub(crate) fn setup_tracing() {
    // Set a default log level if `RUST_LOG` is not set
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info"); // Default to 'info' level
    }
    tracing_subscriber::fmt::init();
}

/// Create a 503 response for a full queue
pub(crate) async fn create_queue_full_response(
    session: &mut Session,
) -> pingora_core::Result<bool> {
    // Set grpc-message header to "Too many requests in the queue"
    // This is meant to be used by a Tonic interceptor to return a gRPC error
    let mut header = ResponseHeader::build(503, None)?;
    header.insert_header("grpc-message", "Too many requests in the queue".to_string())?;
    header.insert_header("grpc-status", RESOURCE_EXHAUSTED_CODE)?;
    session.set_keepalive(None);
    session.write_response_header(Box::new(header.clone()), true).await?;

    let mut error = Error::new(ErrorType::HTTPStatus(503))
        .more_context("Too many requests in the queue")
        .into_in();
    error.set_cause("Too many requests in the queue");

    session.write_response_header(Box::new(header), false).await?;
    Err(error)
}

/// Create a 429 response for too many requests
pub async fn create_too_many_requests_response(
    session: &mut Session,
    max_request_per_second: isize,
) -> pingora_core::Result<bool> {
    // Rate limited, return 429
    let mut header = ResponseHeader::build(429, None)?;
    header.insert_header("X-Rate-Limit-Limit", max_request_per_second.to_string())?;
    header.insert_header("X-Rate-Limit-Remaining", "0")?;
    header.insert_header("X-Rate-Limit-Reset", "1")?;
    session.set_keepalive(None);
    session.write_response_header(Box::new(header), true).await?;
    Ok(true)
}
