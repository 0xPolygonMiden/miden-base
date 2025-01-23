# Miden remote provers

This crate contains protobuf definition for the Miden transaction proving services. It also provides an optional `RemoteTransactionProver`, a client struct that can be used to interact with the prover service from a Rust codebase, to enable it the feature `tx-prover` is needed.

## Features

Description of this crate's features:

| Features     | Description                                                                                                 |
| ------------ | ------------------------------------------------------------------------------------------------------------|
| `std`        | Enable usage of Rust's `std`, use `--no-default-features` for `no-std` support.                             |
| `tx-prover`  | Makes the `RemoteTransactionProver` struct public.                                                          |

## License

This project is [MIT licensed](../LICENSE).
