# Miden transaction prover

A service for generating Miden transaction proofs on-demand. The binary enables spawning workers and a proxy for Miden's remote transaction prover service. 

The worker is a gRPC service that can receive transaction witnesses and returns the proof. It can only handle one request at a time and returns an error if is already in use.

The proxy uses [Cloudflare's Pingora crate](https://crates.io/crates/pingora), which provides features to create a modular proxy. It is meant to handle multiple workers with a queue for each one, load balancing incoming requests in a round-robin manner. Further information about Pingora and its features can be found in the [official GitHub repository](https://github.com/cloudflare/pingora).

Additionally, the library can be imported to utilize `RemoteTransactionProver`, a client struct that can be used to interact with the prover service from a Rust codebase.

## Installation

<!-- The following documentation is documented since the CLI is not on crates-io yet -->
<!-- Install the CLI for production using `cargo`:

```sh
cargo install miden-tx-prover --locked
```

This will install the latest official version of the prover. You can install a specific version using `--version <x.y.z>`:

```sh
cargo install miden-tx-prover --locked --version x.y.z
``` -->

To build the CLI from a local version, from the root of the workspace you can run:

```bash
make install-tx-prover
```

The CLI can be installed from the source code using specific git revisions with `cargo install`. Note that since these aren't official releases we cannot provide much support for any issues you run into, so consider this for advanced users only.

Note that for the prover worker you might need to enable the `testing` feature in case the transactions were executed with reduced proof-of-work requirements (or otherwise, the proving process will fail). This step will also generate the necessary protobuf-related files. You can achieve that by generating the binary with:

```bash
make install-tx-prover-testing
```

## Usage

Once installed, you need to initialize the CLI with:

```bash
miden-tx-prover init
```

This will create the `miden-tx-prover.toml` file in your current directory. This file will hold the configuration for the proxy. You can modify the configuration by changing the host and ports of the services, and add workers. An example of a valid configuration is:

```toml
host = "0.0.0.0"
port = 8082
timeout_secs = 100
connection_timeout_secs = 10
max_queue_items = 10
max_retries_per_request = 1
max_req_per_sec = 5

[[workers]]
host = "0.0.0.0"
port = 8083

[[workers]]
host = "0.0.0.0"
port = 8084
```

To add more workers, you will need to add more items with the `[[workers]]` tags.

To start the worker service you will need to run:

```bash
miden-tx-prover start-worker --host 0.0.0.0 --port 8082
```

This will a worker using the hosts and ports defined in the command options. In case that one of the values is not present, the CLI will default to `0.0.0.0` for the host and `50051` for the port.

To start the proxy service, you will need to run:

```bash
miden-tx-prover-cli start-proxy
```

This command will start the proxy using the workers defined in the configuration file to send transaction witness to prove.

At the moment, when a worker added to the proxy stops working and can not connect to it for a request, the connection is marked as retriable meaning that the proxy will try reaching the following worker in a round-robin fashion. The amount of retries is configurable changing the `max_retries_per_request` value in the configuration file.

Both the worker and the proxy will use the `info` log level by default, but it can be changed by setting the `RUST_LOG` environment variable.

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
miden-tx-prover = { version = "0.6", features = ["async"], default-features = false } # Uses tonic-web-wasm-client transport
miden-tx-prover = { version = "0.6", features = ["async"] } # Uses tonic's Channel transport
```
