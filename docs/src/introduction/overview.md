# Overview
Polygon Miden, a ZK-optimized rollup with client-side proving, will complement Polygon’s set of zero-knowledge solutions aiming to become the internet's value layer.

Unlike many other rollups, Polygon Miden prioritizes ZK-friendliness over EVM compatibility. It also uses a novel state model to exploit the full power of a ZK-centric design. These design decisions allow developers to create applications that are currently difficult or impractical to build on account-based systems. 

This documentation presents detailed guides on:

* Polygon Miden Architecture
* Polygon Miden Network Design [WIP]
* Participating in the Polygon Miden Testnet [WIP]

The Polygon Miden **Architecture** describes Miden's unique state and execution model - an actor-based model with concurrent off-chain state. 

From the **Network Design** perspective, Polygon Miden uses a bi-directional token bridge and verifier contract to ensure computational integrity [WIP]. Miden Nodes act as operators that keep the state and compress state transitions recursively into STARK-proofs. The token bridge on Ethereum verifies these proofs. Users can run Miden Clients to send RPC requests to the Miden Nodes to update the state.

Once we are ready, we will provide an in-depth guide on how developers can participate to use our **testnet**. 
