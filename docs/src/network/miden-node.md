# Miden Node
The Miden Node manages the network state and orchestrates three different modules. At the beginning we don't expect that there will be more than one Miden Node. In the futuer we aim for a decenralized network having multiple nodes managing the network state.

## Transaction Prover
The Transaction Prover can execute and prove transaction execution using the transaction kernel.

Note: This module also exists in the Miden Client.

## Transaction Aggregator
[Needs to be spec'd out] Goal of the Aggregator is to compress proofs by recursion and in fact prove proof verification in the Miden VM.

## Block Producer
[Needs to be spec'd out] Consists of different modules again.

* RPC interface
* Tx Pool (like a mempool)
* State databases
