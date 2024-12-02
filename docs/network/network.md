> **Tip: Recap**
> Polygon Miden network architecture contains a bi-directional token bridge and state machine.
>
> Miden nodes act as operators that maintain state and compress state transitions recursively into STARK-proofs. The token bridge on Ethereum verifies these proofs.
>
> Users can run Miden clients to send RPC requests to the Miden nodes to update the state.
>
> The major components of the Polygon Miden network are:
>
> - Miden clients which represent Miden users.
> - Miden nodes which manage the Miden rollup and compress proofs.
> - A verifier contract which maintains and verifies state on Ethereum.
> - A bridge contract as an entry and exit point for users.

## Overview of the Miden network

![Miden architecture overview](../img/network/architecture-overview.png)

## Miden clients

Users run Miden clients and they provide an interface for wallets representing accounts on Miden.

Miden clients can execute and prove transactions with the tx prover. They can handle arbitrary signature schemes. The default is [Falcon](https://falcon-sign.info/). There is a wallet user interface, a database that stores account data locally, and the required smart contract code that represents the account on Miden.

## Miden nodes

Operators run Miden nodes.

Operators ensure integrity of account, note, and nullifier states - all of which represent the state of Polygon Miden. Operators can execute and prove transactions against single accounts and they can also verify proofs of locally executed transactions.

Furthermore, the operator compresses the proofs in several steps up to a single proof that gets published and verified on the verifier contract. Operators also watch events emitted by the bridge contract to detect deposits and withdrawals.

### Node modules

To manage all of this, Miden nodes have separate modules.

- Tx prover: Executes and proves transactions, like the Miden client.
- Tx aggregator: Batches multiple proofs together to reduce the final state proof size using recursive proving.
- Block producer: exposes the RPC interface to the user and collects transactions in the tx pool and stores the state of Polygon Miden in its three databases (accounts, notes, and nullifiers).

## Verifier contract

This contract on Ethereum verifies proofs sent by the operator running a Miden Node. The proof is verified against the current state root. If accepted the state root changes.

> **Note**
> - Polygon Miden will integrate into the AggLayer.
> - The specific design is not yet finalized.

## Bridge contract

This contract serves as a bridge for Miden users on Ethereum. Users can deposit their tokens and get an equivalent amount minted and sent to the specified address on Polygon Miden.

> **Note**
> - Polygon Miden will integrate into the AggLayer and the Unified Bridge.
> - The specific design is not yet finalized.
