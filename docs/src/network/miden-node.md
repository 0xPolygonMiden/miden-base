# Miden node
The Miden node is the software that processes transactions and creates blocks for the Miden rollup. It manages the network state and orchestrates three different modules.

## Status
The Miden node is still under heavy development and the project can be considered to be in an *alpha* stage. Many features are yet to be implemented and there is a number of limitations which we will lift in the near future.

At this point, we are developing the Miden node for a centralized operator. Thus, the work does not yet include such components as P2P networking and consensus. These will also be added in the future.

## Architecture
The Miden node is made up of three main components, which communicate over gRPC:
- **[RPC](../miden-node/miden-node-rpc.md):** an externally-facing component through which clients can interact with the node. It receives client requests (e.g., to synchronize with the latest state of the chain, or to submit transactions), performs basic validation, and forwards the requests to the appropriate internal components.
- **[Store](../miden-node/miden-node-store.md):** maintains the state of the chain. It serves as the "source of truth" for the chain - i.e., if it is not in the store, the node does not consider it to be part of the chain.
- **[Block Producer](../miden-node/miden-node-block-producer.md):** accepts transactions from the RPC component, creates blocks containing those transactions, and sends them to the store.

All three components can either run as one process, or each component can run in its own process.

The diagram below illustrates high-level design of each component as well as basic interactions between them (components in light-grey are yet to be built).

![Architecture diagram](../diagrams/network/Miden_node.png)
