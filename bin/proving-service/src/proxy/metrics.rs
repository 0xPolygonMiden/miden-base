use std::sync::LazyLock;

use prometheus::{
    Histogram, IntCounter, IntCounterVec, IntGauge, register_histogram, register_int_counter,
    register_int_counter_vec, register_int_gauge,
};

// SAFETY: The `unwrap` calls here are safe because:
// 1. The metrics being registered (gauges, counters, histograms) use hardcoded names and
//    descriptions, which are guaranteed not to conflict within the application.
// 2. Registration errors occur only if there is a naming conflict, which is not possible in this
//    context due to controlled metric definitions.
// 3. Any changes to metric names or types should be carefully reviewed to avoid conflicts.

// QUEUE METRICS
// ================================================================================================

pub static QUEUE_SIZE: LazyLock<IntGauge> =
    LazyLock::new(|| register_int_gauge!("queue_size", "Number of requests in the queue").unwrap());
pub static QUEUE_LATENCY: LazyLock<Histogram> = LazyLock::new(|| {
    register_histogram!(
        "queue_latency",
        "Time (in seconds) requests spend in the queue",
        vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0]
    )
    .unwrap()
});
pub static QUEUE_DROP_COUNT: LazyLock<IntCounter> = LazyLock::new(|| {
    register_int_counter!("queue_drop_count", "Number of requests dropped due to a full queue")
        .unwrap()
});

// WORKER METRICS
// ================================================================================================

pub static WORKER_COUNT: LazyLock<IntGauge> =
    LazyLock::new(|| register_int_gauge!("worker_count", "Total number of workers").unwrap());
pub static WORKER_UNHEALTHY: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "worker_unhealthy",
        "Number of times that each worker was registered as unhealthy",
        &["worker_id"]
    )
    .unwrap()
});
pub static WORKER_BUSY: LazyLock<IntGauge> =
    LazyLock::new(|| register_int_gauge!("worker_busy", "Number of busy workers").unwrap());
pub static WORKER_REQUEST_COUNT: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "worker_request_count",
        "Number of requests processed by each worker",
        &["worker_id"]
    )
    .unwrap()
});

// REQUEST METRICS
// ================================================================================================

pub static REQUEST_FAILURE_COUNT: LazyLock<IntCounter> = LazyLock::new(|| {
    register_int_counter!("request_failure_count", "Number of failed requests").unwrap()
});
pub static REQUEST_RETRIES: LazyLock<IntCounter> = LazyLock::new(|| {
    register_int_counter!("request_retries", "Number of request retries").unwrap()
});
pub static REQUEST_COUNT: LazyLock<IntCounter> = LazyLock::new(|| {
    register_int_counter!("request_count", "Number of requests processed").unwrap()
});
pub static REQUEST_LATENCY: LazyLock<Histogram> = LazyLock::new(|| {
    register_histogram!(
        "request_latency",
        "Time (in seconds) requests take to process",
        vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0]
    )
    .unwrap()
});

// RATE LIMITING METRICS
// ================================================================================================

pub static RATE_LIMITED_REQUESTS: LazyLock<IntCounter> = LazyLock::new(|| {
    register_int_counter!(
        "rate_limited_requests",
        "Number of requests blocked due to rate limiting"
    )
    .unwrap()
});
pub static RATE_LIMIT_VIOLATIONS: LazyLock<IntCounter> = LazyLock::new(|| {
    register_int_counter!("rate_limit_violations", "Number of rate limit violations by clients")
        .unwrap()
});
