use lazy_static::lazy_static;
use prometheus::{
    register_histogram, register_int_counter, register_int_counter_vec, register_int_gauge,
    Histogram, IntCounter, IntCounterVec, IntGauge,
};

lazy_static! {
     // Queue Metrics
     pub static ref QUEUE_SIZE: IntGauge =
     register_int_gauge!("queue_size", "Number of requests in the queue").unwrap();
    pub static ref QUEUE_LATENCY: Histogram =
        register_histogram!("queue_latency", "Time requests spend in the queue", vec![
            0.1, 0.5, 1.0, 2.0, 5.0, 10.0
        ]).unwrap();
    pub static ref QUEUE_DROP_COUNT: IntCounter =
        register_int_counter!("queue_drop_count", "Number of requests dropped due to a full queue").unwrap();

    // Worker Metrics
    pub static ref WORKER_COUNT: IntGauge =
        register_int_gauge!("worker_count", "Number of workers").unwrap();
    pub static ref WORKER_UNHEALTHY: IntCounter =
        register_int_counter!("worker_unhealthy", "Number of unhealthy workers").unwrap();
    pub static ref WORKER_UTILIZATION: IntGauge =
        register_int_gauge!("worker_utilization", "Number of requests being processed by workers").unwrap();
    pub static ref WORKER_REQUEST_COUNT: IntCounterVec =
        register_int_counter_vec!(
            "worker_request_count",
            "Number of requests processed by each worker",
            &["worker_id"]
        ).unwrap();

    // Request Metrics
    pub static ref REQUEST_FAILURE_COUNT: IntCounter =
        register_int_counter!("request_failure_count", "Number of failed requests").unwrap();
    pub static ref REQUEST_RETRIES: IntCounter =
        register_int_counter!("request_retries", "Number of request retries").unwrap();
    pub static ref REQUEST_COUNT: IntCounter =
        register_int_counter!("request_count", "Number of requests processed").unwrap();
    pub static ref REQUEST_LATENCY: Histogram =
        register_histogram!("request_latency", "Time requests take to process", vec![
            0.1, 0.5, 1.0, 2.0, 5.0, 10.0
        ]).unwrap();

    // Rate Limiting Metrics
    pub static ref RATE_LIMITED_REQUESTS: IntCounter =
        register_int_counter!("rate_limited_requests", "Number of requests blocked due to rate limiting").unwrap();
    pub static ref RATE_LIMIT_VIOLATIONS: IntCounter =
        register_int_counter!("rate_limit_violations", "Number of rate limit violations by clients").unwrap();
}
