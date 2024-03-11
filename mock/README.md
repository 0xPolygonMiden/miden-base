# Miden mock

This crate contains utilities to help with testing Miden rollup components.

This crate contains builder and mock functions to create objects for testing. A mock chain which allows the simulation of a rollup in-memory. And some precomputed values to speed up testing.

## Features

| name         | description                                                                                                       |
| ------------ | ----------------------------------------------------------------------------------------------------------------- |
| `std`        | Allows usage of Rust's `std`, for `no-std` compile with `--no-default-features`.                                  |
| `serde`      | Enable `serde` based seralization of most objects. This feature allows saving the mock chains state to a file.    |
| `executable` | Enables the creation of `mock` executable, used to save mock data to a file, which can later on be used by tests. |

## License

This project is [MIT licensed](../LICENSE).
