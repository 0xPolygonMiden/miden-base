# Miden remote provers

This crate contains protobuf definition for the Miden transaction proving services. It also provides an optional `RemoteTransactionProver`, `RemoteBatchProver` and `RemoteBlockProver` structs, which can be used to interact with a remote proving service.

## Features

Description of this crate's features:

| Features      | Description                                                                                                 |
| ------------- | ------------------------------------------------------------------------------------------------------------|
| `std`         | Enable usage of Rust's `std`, use `--no-default-features` for `no-std` support.                             |
| `tx-prover`   | Makes the `RemoteTransactionProver` struct public.                                                          |
| `batch-prover`| Makes the `RemoteBatchProver` struct public.                                                                |
| `block-prover`| Makes the `RemoteBlockProver` struct public.                                                                |

## License

This project is [MIT licensed](../LICENSE).
