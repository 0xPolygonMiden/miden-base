# Overview
Polygon Miden, a ZK-optimized rollup with client-side proving, will complement Polygon’s set of zero-knowledge solutions aiming to become the internet's value layer.

With Polygon Miden, we aim to extend Ethereum's feature set. Ethereum is designed to be a base layer that evolves slowly and provides stability. Rollups allow the creation of new design spaces while retaining the security of Ethereum. This makes a rollup the perfect place to innovate and enable new functionality.

Unlike many other rollups, Polygon Miden prioritizes ZK-friendliness over EVM compatibility. It also uses a novel state model to exploit the full power of a ZK-centric design. These design decisions allow developers to create applications that are currently difficult or impractical to build on account-based systems. 

We extend Ethereum on three core dimensions to attract billions of users: scalability, safety, and privacy.

From the architectural perspective, Polygon Miden uses a bi-directional token bridge and verifier contract to ensure computational integrity. Miden Nodes act as operators that keep the state and compress state transitions recursively into STARK-proofs. The token bridge on Ethereum verifies these proofs. Users can run Miden clients to send RPC requests to the Miden Nodes to update the state.

The major components of Polygon Miden are:

* Miden Clients - represent Miden users
* Miden Nodes - manage the Miden rollup and compress proofs
* Verifier Contract - keeps and verifies state on Ethereum [Not specified yet]
* Bridge Contract - entry and exit point for users [Not specified yet]
