# miden-base

<a href="https://github.com/0xPolygonMiden/miden-base/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
<img src="https://github.com/0xPolygonMiden/miden-base/workflows/CI/badge.svg?branch=main">
<img src="https://img.shields.io/badge/rustc-1.75+-lightgray.svg">
<a href="https://crates.io/crates/miden-base"><img src="https://img.shields.io/crates/v/miden-base"></a>

Common structs for the Miden rollup.

**WARNING:** This project is in an alpha stage. It has not been audited and may contain bugs and
security flaws. This implementation is NOT ready for production use.

## Overview

Miden is a zero-knowledge rollup for high-throughput and private applications. Miden allows users
to prove state changes of local data where the network only tracks a commitment of it. This leads
to privacy and high-throughput. We think, Privacy Scales Better. Users can also let the Operator
prove public state changes as in other known rollups.

Polygon Miden is a modular execution layer that extends Ethereum's capabilities using powerful
features such as parallel transaction execution and client-side proving. With Miden, developers
can create novel, high-throughput, privacy preserving dApps for DeFi, RWA and Autonomous Worlds
using their favorite languages such as Rust and TypeScript.

If you want to join the technical discussion or learn more about the project, please check out

* the [documentation](https://0xpolygonmiden.github.io/miden-base/).
* the [Discord](https://discord.gg/0xpolygondevs)
* the [Repo](https://github.com/0xPolygonMiden)
* the [Roadmap](roadmap.md)

## Status and features

Polygon Miden is currently on release v0.1. This is an early version of the protocol and its
components. We expect to keep making even breaking changes to all components.

At this point, adventurous Pioneers can execute first transactions and send assets to each other.
Polygon Miden doesn't offer all the features one would expect from a zkRollup, yet. During 2024,
we expect to offer gradually more features. Eventually, developers should be able to code any
application they want on Polygon Miden.

### Feature highlights

* **Private accounts**. The Miden Operator only tracks a commitment to any account data in the public database. Users can only execute smart contracts of which they know the interface.
* **Private notes**. Like private accounts, the Miden Operator only tracks a commitment to any notes in the public database. Users need to communicate note details to each other off-chain (via any side channel) in order to consume private notes in transactions.
* **Local transaction execution**. The Miden Client allows for local transaction execution and proving. The Miden Operator verifies the proof and if valid, the state DBs are updated with the new data.
* **Simple smart contracts**. Currently, there are three different smart contracts available. A basic wallet smart contracts to send and receive assets, and fungible and non-fungible faucets to mint and burn assets. All accounts are written in MASM.
* **P2ID, P2IDR and SWAP note scripts**. Currently, there are three different note scripts available. Two different versions of pay-to-id scripts of which P2IDR is reclaimable, and a swap script that allows for simple token swaps.
* **Simple block building**. The Miden Operator running the Miden Node is able to build blocks containing transactions. There is no recursive verification of transactions enabled yet.
* **Maintaining state**. The Miden Node stores all necessary information already in its State DBs and provides this infos via its RPC endpoint.

### Planned features

* **Public accounts**. Polygon Miden will support public smart contracts as know on Ethereum. Code and state of those accounts will be visible to the network and anyone can execute transactions against them.
* **Public notes**. As with public accounts, also public notes will be supported. That means, note data will be publicly stored by the Miden Operator. Note consumption will not be private.
* **Customized smart contracts**. Accounts can expose any interface in the future. This is the Miden version of a smart contract. Account code can be arbitrary complex due to the underlying Turing-complete Miden VM.
* **Customized note scripts**. Users will be able to write their own note scripts using the Miden Client. Note scripts are executed during note consumption and they can be arbitrary complex due to the underlying Turing-complete Miden VM.
* **Network transactions**. Transaction execution and proving can be outsourced to the network and to the Miden Operator. Those transactions will be necessary when it comes to public shared state, and they can be useful if the user's device is not powerful enough to prove transactions efficiently.
* **Rust compiler**. In order to write account code, note or transaction scripts, in Rust, there will be a Rust -> Miden Assembly compiler.
* **Block and epoch proofs**. The Miden Node will recursively verify transactions and in doing so build batches of transactions, blocks and epochs.


## Benefits of Polygon Miden

* Ethereum security
* Developers can build applications infeasible on other systems, e.g.
  * **onchain order book exchange** due to parallel tx execution and updatable transactions
  * **complex, incomplete information games** due to client-side proving and cheap complex computations
  * **safe wallets** due to assets being stored in the accounts and account state can be hidden
* Better privacy properties than on Ethereum - first web2 privacy, later even stronger privacy guarantees
* Transactions can be recalled and updated
* Lower fees due to client-side proving
* dApps on Miden are safe to use due to account abstraction and compile-time safe Rust smart contracts

## Project structure

| Crate                    | Description |
| ------------------------ | ----------- |
| [objects](objects)       | Contains plain old datatype used that comprises the Miden protocol. |
| [miden-lib](miden-lib)   | Contains library code written in Miden assembly used to interact with the VM and implement applications. |
| [miden-tx](miden-tx)     | Contains code to compile and prove transactions using the Miden library and virtual machine. |
| [mock](mock)             | Contains code used for testing, gated behid the `testing` feature flag.|

## Testing

To test the crates contained in this repo, you can run the following command:

```shell
make test
```

Some of the functions in this project are computationally intensive and may take a significant
amount of time to compile and complete during testing. To ensure optimal results we use the `make
test` command. It enables the running of tests in release mode and using specific configurations
replicates the test conditions of the development mode and verifies all debug assertions. For
more information refer to the [Makefile](./Makefile) for the specific commands and configurations
that have been chosen.

## License

This project is [MIT licensed](./LICENSE)
