# State progress (Blockchain)

Miden will start as an Ethereum layer 2. That is why, we mention Ethereum as a settlement layer. However, Miden plans to become its own layer 1 blockchain after consensus and decentralization is worked out. Becoming a layer 2 will ensure a quicker go-to-market to test the Miden hypothesis. This is also why the consensus algorithm is not part of this chapter.

## The purpose of a Miden block
Blocks describe the state progress of the chain. They describe the delta of the global [state](state.md) between two time periods. Blocks aggregate account state changes, or more precisely batches of proofs thereof, together with created and consumed notes. For every block, there is a proof that attests to the correctness of all state transitions it entails.

## Goals of the process
- Miden aims for real time proving. That means when a transaction is included into a block, it is already proven.
- Fast syncing from genesis. Nodes can verify all block proofs, which es exponentially faster than re-executing every single transaction
- ...

## Block building process

<p style="text-align: center;">
    <img src="../img/architecture/execution/execution.png" style="width:30%;" alt="Account diagram"/>
</p>

Regardless of the type of a [transaction](transaction.md), every transaction results in a ZK proof that attests to its correctness. Proven transactions are sent to the block producer. these could come from the end users or from the network transaction builder.

### Transaction batching

To reduce the required space on the blockchain, transaction proofs are not directly put into blocks. First, they are batched together by verifying them in the batch producer. The purpose of the batch producer is to generate a single proof that some number of proven transactions have been verified. This involves recursively verifying individual transaction proofs inside the Miden VM. As with any program that runs in the Miden VM, there is a proof of correct execution running the Miden verifier to verify transaction proofs. This results into a single batch proof.

<p style="text-align: center;">
    <img src="../img/architecture/execution/batching.png" style="width:30%;" alt="Account diagram"/>
</p>

The batch producer processes each transaction proof sequentially and verifies the proofs against the initial and final state root of the accounts affected. If several transactions in the same batch affect one single account, the correct ordering must be ensured.

Also the batch producer takes care to erase ephemeral notes if creation and consumption of this note happens to be in the same block. As an example, there might be two transactions in a batch, `A` and `B`.

```
TX A: Consumes authenticated note Z, produces ephemeral note X.
TX B: Consumes ephemeral note X.
```

From the perspective of the the block in which the batch would be aggregated to, ephemeral note X would have never existed. So the batch producer verifies the correctness of both transactions `A` and `B`, but the Notes DB will never see an entry of note X. That means, the executor of transaction `B` doesn't need to wait until note X exists on the blockchain for it to consume it.

Batch producing is highly parallelizable, and several batches are included into one block

### Block production

Several batch proofs are aggregated into one block. Block production cannot happen in parallel and must be done by the Miden operator. In the future there will be several Miden operators competing on block production. However, the idea of block production is the same - recursive verification. This time, a set of batch proofs is being aggregated into a single block proof. However, the block also entails the commitments to the current global [state](state.md), the new nullifiers, the new state commitments for affected private accounts and created notes, and the full states for all affected public accounts and  created notes. The block proof attests to the correct state transition from one the previous block root to the next and therefore to the change of the global state of Miden. 

<p style="text-align: center;">
    <img src="../img/architecture/execution/block.png" style="width:30%;" alt="Account diagram"/>
</p>

> **Tip: Block contents**
> - **State updates** only contain the hashes of changes. For example, for each updated account, we record a tuple `([account id], [new account hash])`.
> - **ZK Proof** attests that, given a state commitment from the previous block, there was a sequence of valid transactions executed that resulted in the new state commitment, and the output also included state updates.
> - The block also contains full account and note data for public accounts and notes. For example, if account `123` is an updated public account which, in the **state updates** section we'd see a records for it as `(123, 0x456..)`. The full new state of this account (which should hash to `0x456..`) would be included in a separate section.

## Verifying valid block state

To verify that a block describes a valid state transition, the following must be done:

1. Compute hashes of public account and note states.
2. Ensure these hashes match records in the **state updates** section.
3. Verify the included block proof against the following public inputs:
   - State commitment from the previous block.
   - State commitment from the current block.
   - State updates from the current block.

The above can be performed by a verifier contract on Ethereum L1, the Polygon AggLayer, or a decentralized network of Miden nodes.

### Syncing to current state from genesis

The block structure has another nice property. It is very easy for a new node to sync up to the current state from genesis.

The new node needs to do the following:

1. Download only the first part of the blocks (i.e., without full account/note states) starting at the genesis up until the latest block.
2. Verify all ZK proofs in the downloaded blocks. This is super quick (exponentially faster than re-executing original transactions) and can also be done in parallel.
3. Download the current states of account, note, and nullifier databases.
4. Verify that the downloaded current state matches the state commitment in the latest block.

Overall, state sync is dominated by the time needed to download the data.
