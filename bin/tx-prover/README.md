# Miden transaction prover CLI

A CLI to control both workers and proxy for the Miden's remote transaction prover.

The worker is a gRPC service that can receive transaction witnesses and returns the proof. It can only handle one request at a time and returns an error if is already in use.

The proxy uses [Cloudflare's Pingora crate](https://crates.io/crates/pingora), which provides features to create a modular proxy. It is meant to handle multiple workers with a queue for each one. Further information about Pingora and it's features can be found in the [official GitHub repository](https://github.com/cloudflare/pingora).

## Installation

<!-- The following documentation is documented since the CLI is not on crates-io yet -->
<!-- Install the CLI for production using `cargo`:

```sh
cargo install miden-tx-prover-cli --locked
```

This will install the latest official version of the prover. You can install a specific version using `--version <x.y.z>`:

```sh
cargo install miden-tx-prover-worker --locked --version x.y.z
cargo install miden-tx-prover-proxy --locked --version x.y.z
``` -->

To build the CLI from a local version, from the root of the workspace you can run:

```bash
make install-prover-cli
```

The CLI can be installed from the source code using specific git revisions with `cargo install`. Note that since these aren't official releases we cannot provide much support for any issues you run into, so consider this for advanced users only.

Note that for the prover worker you might need to enable the `testing` feature in case the transactions were executed with reduced proof-of-work requirements (or otherwise, the proving process will fail). This step will also generate the necessary protobuf-related files. You can achieve that by generating the binary with:

```bash
make install-prover-cli-testing
```

## Usage

Once installed, you need to initialize the CLI with:

```bash
miden-tx-prover-cli init
```

This will create the `miden-prover-service.toml` file in your current directory. This file will hold the configuration for both workers and the proxy. You can modify the configuration changing the host and ports of the services, and add workers. An example of a valid configuration is:

```toml
[[workers]]
host = "0.0.0.0"
port = 8080

[[workers]]
host = "0.0.0.0"
port = 8081

[proxy]
host = "0.0.0.0"
port = 8082
```

To add more workers, you will need to add more items with the `[[workers]]` tags.

To start the worker service, once you have configurated at least one in the config file, you will need to run:

```bash
RUST_LOG=info miden-tx-prover-cli start-worker
```

This will start all of your configured workers in the same terminal, using the hosts and ports defined in the configuration file.

To start the proxy service, you will need to run:

```bash
RUST_LOG=info miden-tx-prover-cli start-proxy
```

This command will start the proxy using the workers defined in the configuration file to send transaction witness to prove.

At the moment, when a worker added to the proxy stops working and can not connect to it for a request, the connection is marked as retriable meaning that the proxy will try reaching the following worker in a round-robin fashion. The amount of retries is configurable changing the `MAX_RETRIES_PER_REQUEST` constant. To remove the worker from the set of availables, we will need to implement a health check in the worker service.

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
