# Blockchain

The Miden blockchain protocol describes how the [state](state.md) progresses. In Miden, `Block`s are containers that aggregate account state changes and their proofs, together with created and consumed notes. Each `Block` is accompanied by a corresponding proof that attests to the correctness of all state transitions it contains.

`Block`s represent the delta of the global [state](state.md) between two time periods. The current global state can be derived by applying all the blocks to the genesis state.

Miden's blockchain protocol aims for the following:

- **Proven transactions**: All included transactions have already been proven and verified when they reach the block.
- **Fast genesis syncing**: New nodes efficiently sync to the network through a multi-step process:

  1. Download historical `Block`s from genesis to the present.
  2. Verify zero-knowledge proofs for all `Block`s.
  3. Retrieve current state data (accounts, notes, and nullifiers).
  4. Validate that the downloaded state matches the latest `Block`'s state commitment.

This approach enables near-instant blockchain syncing by verifying `Block` proofs rather than re-executing individual transactions, resulting in exponentially faster performance. Consequently, state sync is dominated by the time needed to download the data.

<p style="text-align: center;">
    <img src="../img/architecture/blockchain/execution.png" style="width:70%;" alt="Execution diagram"/>
</p>

## Batch production

To reduce the load on the blockchain, transaction proofs are first aggregated into batches by batch producers instead of being added directly into `Block`s. Batch production is highly parallelizable, and multiple batch producers may run simultaneously.

The purpose of this scheme is to produce a single proof that attests to the validity of a number of transactions. This is achieved by recursively verifying each transaction proof within the Miden VM.

<p style="text-align: center;">
    <img src="../img/architecture/blockchain/batching.png" style="width:50%;" alt="Batch diagram"/>
</p>

The batch producer processes each transaction proof sequentially and verifies each proof against the initial and final state commitment of the affected account. Several rules must be followed to ensure the correctness of the overall protocol:

1. **Ordering of transactions**: If several transactions within the same batch affect a single account, the correct ordering must be enforced. For example, if `Tx1` and `Tx2` both describe state changes of account `A`, then the batch kernel must verify them in the order: `A -> Tx1 -> A' -> Tx2 -> A''`.
2. **Prevention of double spending and duplicate notes**: The batch producer must ensure the uniqueness of all notes across transactions in the batch. This prevents double spending and avoids the situation where duplicate notes, which would share identical nullifiers, are created. Only one of such duplicate notes can later be consumed, as the nullifier will be marked as spent after the first consumption.
3. **Expiration windows**: It is possible to set an expiration window for transactions, which in turn sets an expiration window for the entire batch. For instance, if transaction `A` expires at block `8` and transaction `B` expires at block `5`, then the batch expiration will be set to the minimum of all transaction expirations, which is `5`.

## Block production

To create a `Block`, multiple batches and their respective proofs are aggregated. `Block` production is not parallelizable and must be performed by the Miden operator. In the future, several Miden operators may compete for `Block` production. The schema used for `Block` production is similar to that in batch production—recursive verification. Multiple batch proofs are aggregated into a single `Block` proof. In addition, the `Block` contains:
- The commitments to the current global [state](state.md).
- The newly created nullifiers.
- The new state commitments for affected private accounts.
- The full states for all affected public accounts and newly created notes.

The `Block` proof attests to the correct state transition from the previous `Block` commitment to the next, and therefore to the change in Miden's global state.

<p style="text-align: center;">
    <img src="../img/architecture/blockchain/block.png" style="width:90%;" alt="Block diagram"/>
</p>

> **Tip: Block Contents**
>
> - **State updates**: Contains only the hashes of updated elements. For example, for each updated account, a tuple is recorded as `([account id], [new account hash])`.
> - **ZK Proof**: This proof attests that, given a state commitment from the previous `Block`, a set of valid batches was executed that resulted in the new state commitment.
> - The `Block` also includes the full account and note data for public accounts and notes. For example, if account `123` is a public account that has been updated, you would see a record in the **state updates** section as `(123, 0x456..)`, and the full new state of this account (which should hash to `0x456..`) would be included in a separate section.

## Verifying blocks

To verify that a `Block` corresponds to a valid global state transition, the following steps must be performed:

1. Compute the hashes of public accounts and note states.
2. Ensure that these hashes match the records in the **state updates** section.
3. Verify the included `Block` proof using the following public inputs and output:
   - **Input**: Previous `Block` commitment.
   - **Input**: Set of batch commitments.
   - **Output**: Current `Block` commitment.

These steps can be performed by any verifier (e.g., a contract on Ethereum, Polygon AggLayer, or a decentralized network of Miden nodes).
