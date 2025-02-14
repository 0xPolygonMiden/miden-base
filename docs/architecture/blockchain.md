# Blockchain

`Block`s in Miden are containers that aggregate account state changes and their proofs, together with created and consumed notes. For every `Block`, there is a `Block` proof that attests to the correctness of all state transitions it contains.

From `Block`s one can derive the progress of the chain, by observing the delta of the global [state](state.md) between two time periods.

Miden's `Block` structure aims for the following:

- **Real time proving**: All included transactions have already been proven.
- **Fast Genesis syncing**: New nodes efficiently sync to the network through a multi-step process:

1. Download historical `Block`s from genesis to present
2. Verify zero-knowledge proofs for all `Block`s
3. Retrieve current state data (accounts, notes, and nullifiers)
4. Validate that the downloaded state matches the latest `Block`'s state commitment
 
This approach enables near-instant blockchain syncing by verifying `Block` proofs rather than re-executing individual transactions, resulting in exponentially faster performance. Hence state sync is dominated by the time needed to download the data.

<p style="text-align: center;">
    <img src="../img/architecture/blockchain/execution.png" style="width:70%;" alt="Account diagram"/>
</p>

## Batching

To reduce the load on the blockchain, transactions are firstly aggregated into batches by the `batch_producer`, not directly into `Block`s. Batch producing is highly parallelizable.

The purpose of this shceme is to produce a single proof attesting to the validity of some number of transactions which is done by recursively verifying each transaction proof in the Miden VM.

<p style="text-align: center;">
    <img src="../img/architecture/blockchain/batching.png" style="width:50%;" alt="Account diagram"/>
</p>

The batch producer processes each transaction proof sequentially and verifies the proofs against the initial and final state root of the accounts affected. If several transactions in the same batch affect one single account, the correct ordering must be ensured.

Also the batch producer takes care to erase ephemeral notes if creation and consumption of this note happens to be in the same `Block`. As an example, there might be two transactions in a batch, `A` and `B`.

```
TX A: Consumes authenticated note Z, produces ephemeral note X.
TX B: Consumes ephemeral note X.
```

From the perspective of the the `Block` in which the batch would be aggregated to, ephemeral note X would have never existed. So the batch producer verifies the correctness of both transactions `A` and `B`, but the Notes DB will never see an entry of note X. That means, the executor of transaction `B` doesn't need to wait until note X exists on the blockchain for it to consume it.

## Block production

To create a `Block` multiple batches and their respective proofs are aggregated into a `Block`. 

`Block` production cannot happen in parallel and must be done by the Miden operator. In the future there will be several Miden operators competing for `Block` production. However, the idea of `Block` production is the same - recursive verification. This time, a set of batch proofs is being aggregated into a single `Block` proof. However, the `Block` also entails the commitments to the current global [state](state.md), the new nullifiers, the new state commitments for affected private accounts and created notes, and the full states for all affected public accounts and  created notes. The `Block` proof attests to the correct state transition from one the previous `Block` root to the next and therefore to the change of the global state of Miden. 

<p style="text-align: center;">
    <img src="../img/architecture/blockchain/block.png" style="width:90%;" alt="Account diagram"/>
</p>

> **Tip: Block contents**
> - **State updates** only contain the hashes of changes. For example, for each updated account, we record a tuple `([account id], [new account hash])`.
> - **ZK Proof** attests that, given a state commitment from the previous `Block`, there was a sequence of valid transactions executed that resulted in the new state commitment, and the output also included state updates.
> - The `Block` also contains full account and note data for public accounts and notes. For example, if account `123` is an updated public account which, in the **state updates** section we'd see a records for it as `(123, 0x456..)`. The full new state of this account (which should hash to `0x456..`) would be included in a separate section.

## Verifying blocks

To verify that a `Block` corresponds to a valid state transition, the following must be done:

1. Compute hashes of public accounts and notes states.
2. Ensure that these hashes match records in the **state updates** section.
3. Verify the included `Block` proof against the following public inputs:
   - State commitment from the previous `Block`.
   - State commitment from the current `Block`.
   - State updates from the current `Block`.

The above can be performed by a verifier contract on Ethereum L1, the Polygon AggLayer, or a decentralized network of Miden nodes.
