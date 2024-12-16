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
use tracing_subscriber::{layer::SubscriberExt, Registry};

pub const MIDEN_TX_PROVER: &str = "miden-tx-prover";

const RESOURCE_EXHAUSTED_CODE: u16 = 8;

/// Configures and initializes an OpenTelemetry [TracerProvider] with an OTLP exporter.
///
/// This function sets up a [TracerProvider] to collect, process, and export spans to an
/// OTLP-compatible backend (e.g., OpenTelemetry Collector) using the following configuration:
///
/// ### Configuration Details
/// - **OTLP Exporter**:
///   - The [opentelemetry_otlp::SpanExporter] is configured with the gRPC-based OTLP protocol via
///     the `tonic` library.
///   - It ensures spans are exported to an OTLP endpoint.
///   - Psanics if it fails to initialize.
/// - **Sampler**:
///   - A [Sampler::ParentBased] sampler is used, inheriting the parent's sampling decision for
///     child spans.
///   - The root spans use a [Sampler::TraceIdRatioBased] sampler with a 100% sampling rate (`1.0`).
/// - **ID Generator**:
///   - A `RandomIdGenerator` is used to generate unique trace and span IDs.
/// - **Resource**:
///   - A custom resource is created using the `create_resource` function, including attributes such
///     as the service name, version and schema URL.
/// - **Batch Exporter**:
///   - The spans are exported in batches for improved performance, using the Tokio runtime.
///
/// ### Returns
/// - `Ok(TracerProvider)`: If the tracer provider is successfully initialized.
/// - `Err(String)`: If the OTLP exporter fails to initialize, an error message is returned.
pub(crate) fn init_tracer_provider() -> Result<TracerProvider, String> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .map_err(|e| format!("Failed to create OTLP exporter: {:?}", e))?;

    Ok(TracerProvider::builder()
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(1.0))))
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(create_resource())
        .with_batch_exporter(exporter, runtime::Tokio)
        .build())
}

/// Create a Resource that captures information about the entity for which telemetry is recorded.
fn create_resource() -> Resource {
    Resource::from_schema_url(
        [
            KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
        ],
        SCHEMA_URL,
    )
}

/// Sets up tracing for the CLI using OpenTelemetry and `tracing` subscribers.
///
/// This function integrates `tracing` and OpenTelemetry to provide end-to-end tracing
/// capabilities which are used by the worker and proxy services.
///
/// It performs the following steps:
/// 1. Initializes the OpenTelemetry [TracerProvider] using the [init_tracer_provider] function.
/// 2. Configures a `tracing_opentelemetry::layer` to bridge OpenTelemetry tracing with the
///    `tracing` ecosystem.
/// 3. Sets up a `tracing` subscriber with multiple layers:
///    - **OpenTelemetry Layer**: Exports spans to an OTLP backend.
///    - **Environment Filter Layer**: Dynamically controls log levels using the `RUST_LOG`
///      environment variable.
///    - **Formatting Layer**: Outputs human-readable logs to standard output.
/// 4. Registers the configured subscriber as the global default for the `tracing` library.
///
/// ### Returns
/// - `Ok(())`: If the tracing setup is successful.
/// - `Err(String)`: If an error occurs during the setup, an error message is returned.
pub(crate) fn setup_tracing() -> Result<(), String> {
    let provider = init_tracer_provider()?;

    let tracer = provider.tracer(MIDEN_TX_PROVER);

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let subscriber = Registry::default()
        .with(telemetry)
        .with(tracing_subscriber::filter::EnvFilter::from_default_env())
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
