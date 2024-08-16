# Miden Rollup protocol

[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/0xPolygonMiden/miden-base/blob/main/LICENSE)
[![test](https://github.com/0xPolygonMiden/miden-base/actions/workflows/test.yml/badge.svg)](https://github.com/0xPolygonMiden/miden-base/actions/workflows/test.yml)
[![build](https://github.com/0xPolygonMiden/miden-base/actions/workflows/build.yml/badge.svg)](https://github.com/0xPolygonMiden/miden-base/actions/workflows/build.yml)
[![RUST_VERSION](https://img.shields.io/badge/rustc-1.80+-lightgray.svg)](https://www.rust-lang.org/tools/install)
[![GitHub Release](https://img.shields.io/github/release/0xPolygonMiden/miden-base)](https://github.com/0xPolygonMiden/miden-base/releases/)

Description and core structures for the Miden Rollup protocol.

**WARNING:** This project is in an alpha stage. It has not been audited and may contain bugs and security flaws. This implementation is NOT ready for production use.

## Overview

Miden is a zero-knowledge rollup for high-throughput and private applications. Miden allows users to execute and prove transactions locally (i.e., on their devices) and commit only the proofs of the executed transactions to the network.

If you want to join the technical discussion or learn more about the project, please check out

* the [Documentation](https://docs.polygon.technology/miden/).
* the [Discord](https://discord.gg/0xpolygondevs)
* the [Repo](https://github.com/0xPolygonMiden)
* the [Roadmap](roadmap.md)

## Status and features

Polygon Miden is currently on release v0.5. This is an early version of the protocol and its components. We expect to keep making changes (including breaking changes) to all components.

### Feature highlights

- **Private accounts**. The Miden Operator tracks only commitments to account data in the public database. The users are responsible for keeping track of the state of their accounts.
- **Public accounts**. With public accounts users are be able to store the entire state of their accounts on-chain, thus, eliminating the need to keep track of account states locally (albeit by sacrificing privacy and at a higher cost).
- **Private notes**. Like with private accounts, the Miden Operator tracks only commitments to notes in the public database. Users need to communicate note details to each other via side channels.
- **Public notes**. With public notes, the users are be able to store all note details on-chain, thus, eliminating the need to communicate note details via side-channels.
- **Local transactions**. Users can execute and prove transactions locally on their devices. The Miden Operator verifies the proofs and if the proofs are valid, updates the state of the rollup accordingly.
- **Standard account**. Users can create accounts using a small number of standard account interfaces (e.g., basic wallet). In the future, the set of standard smart contracts will be expanded.
- **Standard notes**. Can create notes using standardized note scripts such as pay-to-ID (`P2ID`) and atomic swap (`SWAP`). In the future, the set of standardized notes will be expanded.

### Planned features

- **More storage types**. In addition to simple storage slots and storage maps, the accounts will be able to store data in storage arrays.
- **Transaction recency conditions**. Users will be able to specify how close to the chain tip their transactions are to be executed. This will enable things like rate limiting and oracles.
- **Delegated note inclusion proofs**. By delegating note inclusion proofs, users will be able to create chains of dependent notes which are would be included into a block as a single batch.
- **Network transactions**. Users will be able to create notes intended for network execution. Such notes will be included into transactions executed and proven by the Miden operator.
- **Encrypted notes**. With encrypted notes users will be able to put all note details on-chain, but the data contained withing the notes would be encrypted with the recipients key.

## Project structure

| Crate                  | Description                                                                     |
| ---------------------- | ------------------------------------------------------------------------------- |
| [objects](objects)     | Contains core components defining the Miden rollup protocol.                    |
| [miden-lib](miden-lib) | Contains the code of the Miden rollup kernels and standardized smart contracts. |
| [miden-tx](miden-tx)   | Contains tool for creating, executing, and proving Miden rollup transaction.    |
| [bench-tx](bench-tx)   | Contains transaction execution and proving benchmarks.                          |

## Make commands

We use `make` to automate building, testing, and other processes. In most cases, `make` commands are just wrappers around `cargo` commands with specific arguments. You can view the list of available commands in the [Makefile](Makefile), or just run the following command:

```shell
make
```

## Testing

To test the crates contained in this repo you can use Make to run the following command present in our [Makefile](Makefile):

```shell
make test
```

Some of the functions in this project are computationally intensive and may take a significant amount of time to compile and complete during testing. To ensure optimal results we use the `make test` command. It enables the running of tests in release mode and using specific configurations replicates the test conditions of the development mode and verifies all debug assertions.

## License

This project is [MIT licensed](./LICENSE)
