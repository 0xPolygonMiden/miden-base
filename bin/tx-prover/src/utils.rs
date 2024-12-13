use std::time::Duration;

use opentelemetry::{trace::TracerProvider as _, KeyValue};
use opentelemetry_sdk::{
    runtime,
    trace::{RandomIdGenerator, Sampler, TracerProvider},
    Resource,
};
use opentelemetry_semantic_conventions::{
    resource::{SERVICE_NAME, SERVICE_VERSION},
    SCHEMA_URL,
};
use pingora::{http::ResponseHeader, Error, ErrorType};
use pingora_proxy::Session;
use tonic::transport::Channel;
use tonic_health::pb::health_client::HealthClient;
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, Registry};

pub const TRACING_TARGET_NAME: &str = "miden-tx-prover";

const RESOURCE_EXHAUSTED_CODE: u16 = 8;

// Construct TracerProvider for OpenTelemetryLayer
pub(crate) fn init_tracer_provider() -> TracerProvider {
    let exporter = create_span_exporter();

    TracerProvider::builder()
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(1.0))))
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(create_resource())
        .with_batch_exporter(exporter, runtime::Tokio)
        .build()
}

// Create a SpanExporter
fn create_span_exporter() -> opentelemetry_otlp::SpanExporter {
    opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("Failed to build SpanExporter")
}

// Create a Resource that captures information about the entity for which telemetry is recorded.
fn create_resource() -> Resource {
    Resource::from_schema_url(
        [
            KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
        ],
        SCHEMA_URL,
    )
}

// Setup tracing subscriber
pub(crate) fn setup_tracing(provider: TracerProvider) -> Result<(), String> {
    let tracer = provider.tracer(TRACING_TARGET_NAME);

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let subscriber = Registry::default()
        .with(telemetry)
        .with(tracing_subscriber::filter::LevelFilter::from_level(Level::INFO))
        .with(tracing_subscriber::fmt::layer());

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| format!("Failed to set subscriber: {:?}", e))
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

/// Create a 200 response for updated workers
///
/// It will set the X-Worker-Count header to the number of workers.
pub async fn create_workers_updated_response(
    session: &mut Session,
    workers: usize,
) -> pingora_core::Result<bool> {
    let mut header = ResponseHeader::build(200, None)?;
    header.insert_header("X-Worker-Count", workers.to_string())?;
    session.set_keepalive(None);
    session.write_response_header(Box::new(header), true).await?;
    Ok(true)
}

/// Create a 400 response with an error message
///
/// It will set the X-Error-Message header to the error message.
pub async fn create_response_with_error_message(
    session: &mut Session,
    error_msg: String,
) -> pingora_core::Result<bool> {
    let mut header = ResponseHeader::build(400, None)?;
    header.insert_header("X-Error-Message", error_msg)?;
    session.set_keepalive(None);
    session.write_response_header(Box::new(header), true).await?;
    Ok(true)
}

/// Create a gRPC [HealthClient] for the given worker address.
///
/// It will panic if the worker URI is invalid.
pub async fn create_health_check_client(
    address: String,
    connection_timeout: Duration,
    total_timeout: Duration,
) -> Result<HealthClient<Channel>, String> {
    Channel::from_shared(format!("http://{}", address))
        .map_err(|err| format!("Invalid format for worker URI: {}", err))?
        .connect_timeout(connection_timeout)
        .timeout(total_timeout)
        .connect()
        .await
        .map(HealthClient::new)
        .map_err(|err| format!("Failed to create health check client for worker: {}", err))
}
