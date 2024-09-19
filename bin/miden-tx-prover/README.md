# miden-tx-prover-service

A service that generates proofs on-demand.

## Installation

To install the prover service, run:

```bash
make install-prover
```

This step will also generate the necessary protobuf-related files.

### Running the Service

Once installed, you can run the service with:

```bash
RUST_LOG=info miden-tx-prover-service
```

By default, the server will start on `0.0.0.0:50051`. You can change this and the log level by setting the following environment variables:

```bash
PROVER_SERVICE_HOST=<your-host>
PROVER_SERVICE_PORT=<your-port>
RUST_LOG=<log-level>
```
