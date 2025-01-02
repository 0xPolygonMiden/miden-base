# Miden transaction prover

A service for generating Miden transaction proofs on-demand. The binary enables spawning workers and a proxy for Miden's remote transaction prover service. 

The worker is a gRPC service that can receive transaction witnesses and returns the proof. It can only handle one request at a time and returns an error if is already in use.

The proxy uses [Cloudflare's Pingora crate](https://crates.io/crates/pingora), which provides features to create a modular proxy. It is meant to handle multiple workers with a queue, assigning a worker to each request and retrying if the worker is not available. Further information about Pingora and its features can be found in the [official GitHub repository](https://github.com/cloudflare/pingora).

Additionally, the library can be imported to utilize `RemoteTransactionProver`, a client struct that can be used to interact with the prover service from a Rust codebase.

## Installation

To build the service from a local version, from the root of the workspace you can run:

```bash
make install-tx-prover
```

The CLI can be installed from the source code using specific git revisions with `cargo install`. Note that since these aren't official releases we cannot provide much support for any issues you run into, so consider this for advanced users only.

Note that for the prover worker you might need to enable the `testing` feature in case the transactions were executed with reduced proof-of-work requirements (or otherwise, the proving process will fail). This step will also generate the necessary protobuf-related files. You can achieve that by generating the binary with:

```bash
make install-tx-prover-testing
```

## Worker

To start the worker service you will need to run:

```bash
miden-tx-prover start-worker --host 0.0.0.0 --port 8082
```

This will spawn a worker using the hosts and ports defined in the command options. In case that one of the values is not present, it will default to `0.0.0.0` for the host and `50051` for the port.

## Proxy

First, you need to create a configuration file for the proxy with:

```bash
miden-tx-prover init
```

This will create the `miden-tx-prover.toml` file in your current directory. This file will hold the configuration for the proxy. You can modify the configuration by changing the host and ports of the services, the maximum size of the queue, among other options. An example configuration is:

```toml
# Host of the proxy server
host = "0.0.0.0"
# Port of the proxy server
port = 8082
# Timeout for a new request to be completed
timeout_secs = 100
# Timeout for establishing a connection to the worker
connection_timeout_secs = 10
# Maximum amount of items that a queue can handle
max_queue_items = 10
# Maximum amount of retries that a request can take
max_retries_per_request = 1
# Maximum amount of requests that a given IP address can make per second
max_req_per_sec = 5
# Time to wait before checking the availability of workers
available_workers_polling_time_ms = 20
# Interval to check the health of the workers
health_check_interval_secs = 1
# Host of the metrics server
prometheus_host = "127.0.0.1"
# Port of the metrics server
prometheus_port = 6192
```

Then, to start the proxy service, you will need to run:

```bash
miden-tx-prover start-proxy [worker1] [worker2] ... [workerN]
```

This command will start the proxy using the workers passed as arguments. The workers should be in the format `host:port`. If no workers are passed, the proxy will start without any workers and will not be able to handle any requests until one is added through the `miden-tx-prover add-worker` command.

At the moment, when a worker added to the proxy stops working and can not connect to it for a request, the connection is marked as retriable meaning that the proxy will try reaching another worker. The number of retries is configurable via the `max_retries_per_request` value in the configuration file.

## Updating workers on a running proxy

To update the workers on a running proxy, two commands are provided: `add-worker` and `remove-worker`. These commands will update the workers on the proxy and will not require a restart. To use these commands, you will need to run:

```bash
miden-tx-prover add-worker [worker1] [worker2] ... [workerN]
miden-tx-prover remove-worker [worker1] [worker2] ... [workerN]
```
For example:

```bash
# To add 0.0.0.0:8085 and 200.58.70.4:50051 to the workers list:
miden-tx-prover add-workers 0.0.0.0:8085 200.58.70.4:50051
# To remove 158.12.12.3:8080 and 122.122.6.6:50051 from the workers list:
miden-tx-prover remove-workers 158.12.12.3:8080 122.122.6.6:50051
```

Note that, in order to update the workers, the proxy must be running in the same computer as the command is being executed because it will check if the client address is localhost to avoid any security issues.

### Health check

The worker service implements the [gRPC Health Check](https://grpc.io/docs/guides/health-checking/) standard, and includes the methods described in this [official proto file](https://github.com/grpc/grpc-proto/blob/master/grpc/health/v1/health.proto).

The proxy service uses this health check to determine if a worker is available to receive requests. If a worker is not available, it will be removed from the set of workers that the proxy can use to send requests.

## Logging and Tracing

The service uses the [`tracing`](https://docs.rs/tracing/latest/tracing/) crate for both logging and distributed tracing, providing structured, high-performance logs and trace data.

By default, logs are written to `stdout` and the default logging level is `info`. This can be changed via the `RUST_LOG` environment variable. For example:

```
export RUST_LOG=debug
```

For tracing, we use OpenTelemetry protocol. By default, traces are exported to the endpoint specified by `OTEL_EXPORTER_OTLP_ENDPOINT` environment variable. To consume and visualize these traces we can use Jaeger or any other OpenTelemetry compatible consumer.

The simplest way to install Jaeger is by using a [Docker](https://www.docker.com/) container. To do so, run:

```bash
docker run -d -p4317:4317 -p16686:16686 jaegertracing/all-in-one:latest
```

Then access the Jaeger UI at `http://localhost:16686/`.

If Docker is not an option, Jaeger can also be set up directly on your machine or hosted in the cloud. See the [Jaeger documentation](https://www.jaegertracing.io/docs/) for alternative installation methods.

## Metrics

The proxy includes a service that exposes metrics to be consumed by [Prometheus](https://prometheus.io/docs/introduction/overview/). This service is always enabled and uses the host and port defined in the `miden-tx-prover.toml` file.

The metrics architecture works by having the proxy expose metrics at an endpoint (`/metrics`) in a format Prometheus can read. Prometheus periodically scrapes this endpoint, adds timestamps to the metrics, and stores them in its time-series database. Grafana then queries Prometheus to retrieve and visualize these metrics, allowing you to create dashboards and set up alerts based on the stored data.

The simplest way to install Prometheus and Grafana is by using Docker containers. To do so, run:

```bash
docker run \
    -d \
    -p 9090:9090 \
    -v /path/to/prometheus.yml:/etc/prometheus/prometheus.yml \
    prom/prometheus

docker run -d -p 3000:3000 --name grafana grafana/grafana-enterprise:latest
```

In case that Docker is not an option, Prometheus and Grafana can also be set up directly on your machine or hosted in the cloud. See the [Prometheus documentation](https://prometheus.io/docs/prometheus/latest/getting_started/) and [Grafana documentation](https://grafana.com/docs/grafana/latest/setup-grafana/) for alternative installation methods.

A prometheus configuration file is provided in this repository, you will need to modify the `scrape_configs` section to include the host and port of the proxy service.

Then, to add the new Prometheus collector as a datasource for Grafana, you can [follow this tutorial](https://grafana.com/docs/grafana-cloud/connect-externally-hosted/existing-datasource/). A Grafana dashboard under the name `proxy_grafana_dashboard.json` is provided, see this [link](https://grafana.com/docs/grafana/latest/dashboards/build-dashboards/import-dashboards/) to import it. Otherwise, you can [create your own dashboard](https://grafana.com/docs/grafana/latest/getting-started/build-first-dashboard/) using the metrics provided by the proxy and export it by following this [link](https://grafana.com/docs/grafana/latest/dashboards/share-dashboards-panels/#export-a-dashboard-as-json).

## Features

Description of this crate's feature:

| Features     | Description                                                                                                 |
| ------------ | ------------------------------------------------------------------------------------------------------------|
| `std`        | Enable usage of Rust's `std`, use `--no-default-features` for `no-std` support.                             |
| `concurrent` | Enables concurrent code to speed up runtime execution.                                                      |
| `async`      | Enables the `RemoteTransactionProver` struct, that implements an async version of `TransactionProver` trait.|
| `testing`    | Enables testing utilities and reduces proof-of-work requirements to speed up tests' runtimes.               |

### Using RemoteTransactionProver
To use the `RemoteTransactionProver` struct, enable `async`. Additionally, when compiling for `wasm32-unknown-unknown`, disable default features.

```
[dependencies]
miden-tx-prover = { version = "0.7", features = ["async"], default-features = false } # Uses tonic-web-wasm-client transport
miden-tx-prover = { version = "0.7", features = ["async"] } # Uses tonic's Channel transport
```
