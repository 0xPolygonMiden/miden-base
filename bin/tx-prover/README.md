# Miden transaction prover

A service for generating Miden transaction proofs on-demand. It is split in two binaries: worker
and proxy.

The worker is a gRPC service that can receive transaction witnesses and returns the proof. It can
only handle one request at a time and returns an error if is already in use.

The proxy uses [Cloudflare's Pingora crate](https://crates.io/crates/pingora), which provides features to create a modular proxy. It is
meant to handle multiple workers with a queue for each one. Further information about Pingora and it's features can be found in the [official GitHub repository](https://github.com/cloudflare/pingora).

## Installation

Install the prover worker and proxy binaries for production using `cargo`:

```sh
cargo install miden-tx-prover-worker --locked
cargo install miden-tx-prover-proxy --locked
```

This will install the latest official version of the prover. You can install a specific version using `--version <x.y.z>`:

```sh
cargo install miden-tx-prover-worker --locked --version x.y.z
cargo install miden-tx-prover-proxy --locked --version x.y.z
```

To install both services at once from the source code using specific git revisions you can use `cargo install`. Note that since these aren't official releases we cannot provide much support for any issues you run into, so consider this for advanced users only:

```sh
# Install from a specific branch
cargo install --locked --git https://github.com/0xPolygonMiden/miden-base miden-tx-prover-worker --branch <branch>
cargo install --locked --git https://github.com/0xPolygonMiden/miden-base miden-tx-prover-proxy --branch <branch>

# Install a specific tag
cargo install --locked --git https://github.com/0xPolygonMiden/miden-base miden-tx-prover-worker --tag <tag>
cargo install --locked --git https://github.com/0xPolygonMiden/miden-base miden-tx-prover-proxy --tag <tag>

# Install a specific git revision
cargo install --locked --git https://github.com/0xPolygonMiden/miden-base miden-tx-prover-worker --rev <git-sha>
cargo install --locked --git https://github.com/0xPolygonMiden/miden-base miden-tx-prover-proxy --rev <git-sha>
```
If you want to build from a local version, from the root of the workspace you can run:

```bash
make install-prover-worker
make install-prover-proxy
```

Note that for the prover worker you might need to enable the `testing` feature in case the transactions were executed with reduced proof-of-work requirements (or otherwise, the proving process will fail). This step will also generate the necessary protobuf-related files. You can achieve that by generating the binary with:

```bash
make install-prover-worker-testing
```

## Running the Service

Once installed, you can run the worker with:

```bash
RUST_LOG=info miden-tx-prover-worker
```

By default, the server will start on `0.0.0.0:50051`. You can change this and the log level by setting the following environment variables:

```bash
PROVER_SERVICE_HOST=<your-host>
PROVER_SERVICE_PORT=<your-port>
RUST_LOG=<log-level>
```

And to run the proxy:

```bash
RUST_LOG=info miden-tx-prover-proxy
```

By default, the server will start on `0.0.0.0:6188`. This can be changed by setting:

```bash
PROXY_HOST=<your-host>
PROXY_PORT=<your-port>
```

Also, it is mandatory to set at least one prover worker by setting the `PROVER_WORKERS` env var:

```bash
PROVER_WORKERS=<your-backends>

# For only 1 backend
PROVER_WORKERS="0.0.0.0:50051"

# For multiple backends
PROVER_WORKERS="0.0.0.0:8080,0.0.0.0:50051,165.75.2.4:1010,10.2.2.1:9999"
```

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
