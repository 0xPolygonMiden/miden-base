# Transaction

A `Transaction` in Miden is the state transition of a single account. A `Transaction` takes as input a single [account](accounts.md) and zero or more [notes](notes.md), and outputs the same account with an updated state, together with zero or more notes. `Transaction`s in Miden are executed as Miden VM programs, resulting in the generation of a zero-knowledge proof.

![Transaction diagram](../img/architecture/transaction/transaction-diagram.png)

Compared to most blockchains, where a `Transaction` typically involves more than one account (e.g., sender and receiver) a `Transaction` in Miden involves a single account. To illustrate, Alice sends 5 ETH to Bob. In Miden, sending 5 ETH from Alice to Bob takes two `Transaction`s, one in which Alice creates a note containing 5 ETH and one in which Bob consumes that note and receives the 5 ETH. This model removes the need for a global lock on the blockchain state enabling Miden to process `Transaction`s in parallel.

## What is the purpose of Miden's transaction model?

Miden's `Transaction` model aims for the following:

- **Parallel transaction execution**: Accounts can update their state independently from each other and in parallel.
- **Private transaction execution**: Client-side `Transaction` proving allows the network to verify `Transaction`s with zero knowledge.

## Transaction types

There are two types of `Transaction`s in Miden: **local transactions** and **network transactions** [not yet implemented].

### Local transaction

users transition their account's state locally using the Miden VM and generate a `Transaction` proof that can be verified by the network, which we call **client-side proving**. The network then only has to verify the proof and to change the global parts of the state to apply the state transition.

They are useful, because:

1. They enable privacy as neither the account state nor account code are needed to verify the zk-proof. Public inputs are only commitments and block information that are stored on-chain.
2. They are cheaper (i.e., lower in fees) as the execution of the state transition and the generation of the zk-proof are already made by the users. Hence **privacy is the cheaper option on Miden**.
3. They allow arbitrary complex computation to be done. The proof size doesn't grow linearly with the complexity of the computation. Hence there is no gas limit for client-side proving.

### Network transaction

In **network transactions**, the Miden operator executes the `Transaction` and generates the proof. Miden uses network `Transaction`s for smart contracts with public shared state. This type of `Transaction` is quite similar to the ones in traditional blockchains (e.g., Ethereum).

They are useful, because:

1. For public shared state of smart contracts. Network `Transaction`s allow orchestrated state changes of public smart contracts without race conditions.
2. Smart contracts should be able to be executed autonomously, ensuring liveness. Local `Transaction`s require a user to execute and prove, but in some cases a smart contract should be able to execute when certain conditions are met.
3. Clients may not have sufficient resources to generate zk-proofs.

![Local vs network transactions](../img/architecture/transaction/local-vs-network-transaction.png)

The ability to facilitate both, local and network `Transaction`s, is what differentiates Miden from other blockchains. Local `Transaction` execution and proving can happen in parallel as for most `Transaction`s there is no need for public state changes. This increases the network's throughput tremendously and provides privacy. Network `Transaction`s on the other hand enable autonomous smart contracts and public shared state.

## Transaction lifecycle

Every `Transaction` describes the process of an account changing its state. This process described as a Miden-VM program resulting in a zero-knowledge proof of correct execution.

![Transaction execution process](../img/architecture/transaction/transaction-execution-process.png)

### Prerequisites

To execute a `Transaction`, the executor must have complete knowledge of the account state and the notes used as `Transaction` inputs, specifically:

- Notes: A `Transaction` can only consume notes if the full note data is known. For private notes, the data can not be fetched from the blockchain and must be received otherwise.
- Foreign account data: Any foreign account data accessed during a `Transaction`, whether private or public, must be available beforehand. There is no need to know the full account storage, but the data necessary for the `Transaction`, e.g., the key/value pair that is read and the corresponding storage state root.
- Blockchain state: The current `BlockHeader` and `ChainMMR` used to authenticate input notes must be retrieved from the Miden operator before execution.

> **Info**
> - Usually, notes that are consumed in a `Transaction` must be recorded on-chain in order for the `Transaction` to succeed. However, in Miden there is the concept of ephemeral notes that can be consumed in a `Transaction` before registered on-chain. This allows for the executor to consume notes before they reach the blockchain which is useful for sub-second orders.
> - There is no nullifier-check during a `Transaction`. Nullifiers are checked by the Miden operator during `Transaction` verification. So at the `Transaction` level, there is "double spending." If a note was already spent, i.e. there exists a nullifier for that note, the whole `Transaction` will fail when submitted to the network.

### Transaction execution flow

![Transaction execution process](../img/architecture/transaction/transaction-program.png)

1. **Prologue**: On-chain commitments are validated against provided data.
2. **Note processing**: Input notes are executed sequentially against the account, following a selected order.
    - Notes must be consumed fully (all assets must be transferred to the account or to another note).
    - The note script must be executed in full with the provided note inputs and `TransactionArguments`. `TransactionArguments` can be injected by the executor for each note at runtime.
3. **Transaction script execution (optional)**: A `Transaction` script, if present, is executed.
    - This script can sign the `Transaction` or directly interact with the account without using notes.
4. **Epilogue**:
    - The account state is updated.
    - New notes are created (optional), transferring assets from the account to the newly created notes.
    - Execution completes, resulting in an updated account state and a generated zk-proof.

## Example of consuming a note in a transaction

To illustrate, consider a **basic wallet account** that exposes a `receive_asset` function. When a note is consumed, it's script executes against the account, calling `receive_asset`. This transfers the assets contained in the note to the account.

### Note consumption conditions

Note creators can impose conditions on who can consume a note. These restrictions are enforced by the note script, which must be fully executed by the consuming account. For instance:

- A **P2ID** note verifies that the executing account's ID matches the expected account ID from the note inputs.
- A **Swap** note allows asset exchange based on predefined conditions. Example:
    - The note's consumption condition is defined as "anyone can consume this note to take X units of asset A if they simultaneously create a note sending Y units of asset B back to the creator."
    - If an executor wants to buy only a fraction `(X-m)` of asset A, they provide this amount via `TransactionArguments`.
    - The note script then enforces the correct transfer:
        - A new note is created returning `Y-((m*Y)/X)` of asset B to the sender.
        - A second note is created, holding the remaining `(X-m)` of asset A for future consumption.

### Account Interaction via Transaction Script

Not all `Transaction`s require notes. For example, the owner of a faucet can mint new tokens using only a `Transaction` script, without interacting with external notes.

If the `Transaction` succeeds, a proof is generated of the correct `Transaction` execution. This proof together with the corresponding data needed for verification and updates on the global state can then be submitted and processed by the network.

> **More info**
> - One of the main reasons for separating execution and proving steps is to allow _stateless provers_; i.e., the executed `Transaction` has all the data it needs to re-execute and prove a `Transaction` without database access. This supports easier proof-generation distribution.
> - It is possible to set `Transaction` expiration heights and in doing so, to define a block height until a `Transaction` should be included into a block. If the `Transaction` is expired, the resulting account state change is not valid and the `Transaction` can not be verified anymore.
> - Note and `Transaction` scripts can read the state of foreign accounts during execution. This is called foreign procedure invocation. For example the price of an asset for the SWAP script might depend on a certain value stored in the oracle account.
