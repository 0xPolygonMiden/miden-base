# Miden transaction prover

A service for generating Miden transaction proofs on-demand.

## Installation

Install the prover binary for production using `cargo`:

```sh
cargo install miden-tx-prover --locked
```

This will install the latest official version of the prover. You can install a specific version using `--version <x.y.z>`:

```sh
cargo install miden-tx-prover --locked --version x.y.z
```

You can also use `cargo` to compile the prover service from the source code if for some reason you need a specific git revision. Note that since these aren't official releases we cannot provide much support for any issues you run into, so consider this for advanced users only. The incantation is a little different as you'll be targetting this repo instead:

```sh
# Install from a specific branch
cargo install --locked --git https://github.com/0xPolygonMiden/miden-base miden-tx-prover --branch <branch>

# Install a specific tag
cargo install --locked --git https://github.com/0xPolygonMiden/miden-base miden-tx-prover --tag <tag>

# Install a specific git revision
cargo install --locked --git https://github.com/0xPolygonMiden/miden-base miden-tx-prover --rev <git-sha>
```

If you want to build the prover from a local version, from the root of the workspace you can run:

```bash
make install-prover
```

This step will also generate the necessary protobuf-related files.

### Running the Service

Once installed, you can run the service with:

```bash
RUST_LOG=info miden-tx-prover
```

By default, the server will start on `0.0.0.0:50051`. You can change this and the log level by setting the following environment variables:

```bash
PROVER_SERVICE_HOST=<your-host>
PROVER_SERVICE_PORT=<your-port>
RUST_LOG=<log-level>
```

### Using RemoteTransactionProver
To use the `RemoteTransactionProver` struct, enable `async`. Additionally, when compiling for `wasm32-unknown-unknown`, disable default features.

```
[dependencies]
miden-tx-prover = { version = "0.6", features = ["async"], default-features = false } # Uses tonic-web-wasm-client transport
miden-tx-prover = { version = "0.6", features = ["async"] } # Uses tonic's Channel transport
```
