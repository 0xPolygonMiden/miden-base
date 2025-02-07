# Transaction

A `Transaction` in Miden is the state transition of a single `Account`. A `Transaction` takes a single [account](accounts.md) and zero or more [notes](notes.md), and outputs the same `Account` in a new state, together with zero or more [notes]. `Transaction`s in Miden are executed as Miden VM programs, which means that a zero-knowledge proof is generated as a result of each `Transaction`.

![Transaction diagram](../img/architecture/transaction/transaction-diagram.png)

Compared to most other blockchains, where a transaction typically involves more than one account, e.g., the sender and the receiver. For example, Alice sends 5 ETH to Bob. In Miden, sending 5 ETH from Alice to Bob takes two transactions, one in which Alice creates a note containing 5 ETH and one in which Bob consumes that note and receives the ETH. This model enables Miden to treat transactions (single state transitions) asynchronously and therefore allows accounts to prove their own state transitions independently from each other - no global lock is needed and transactions can happen in parallel.

## What is the purpose of Miden's transaction model?

Miden's `Transaction` model aims for the following:

- **Parallel transaction execution**: `Accounts` can change their state independently from each other and in parallel.
- **Private transaction execution**: Client-side transaction execution and proving allows the network to verify transactions with zero knowledge.

## Transaction types

There are two types of transactions in Miden: **local transactions** and **network transactions** [not yet implemented].
There are two types of transactions in Miden: **local transactions** and **network transactions** [not yet implemented].

In **local transactions**, users transition their account's state locally within the Miden VM and generate a transaction proof that can be verified by the network, called client-side proving. The network only verifies the proof and changes the global parts of the state.
In **local transactions**, users transition their account's state locally within the Miden VM and generate a transaction proof that can be verified by the network, called client-side proving. The network only verifies the proof and changes the global parts of the state.

They are useful, because:

1. They are cheaper (i.e., lower in fees) as the execution of the state transition and the generation of the zk-proof are already made by the users. Hence **privacy is the cheaper option on Miden**.
2. They allow arbitrary complex computation to be done. The proof size doesn't grow linearly with the complexity of the computation. Hence there is no gas limit for client-side execution and proving.
2. They allow arbitrary complex computation to be done. The proof size doesn't grow linearly with the complexity of the computation. Hence there is no gas limit for client-side execution and proving.
3. They enable privacy as neither the account state nor account code are needed to verify the zk-proof. Public inputs are only commitments and block information that are stored on-chain.

In **network transactions**, the Miden operator executes the transaction and generates the proof. Miden uses network transactions for smart contracts with public shared state. This type of `Transaction` is quite similar to the ones in traditional blockchains (e.g., Ethereum).

They are useful, because:

1. Smart contracts should be able to be executed autonomously. Local transactions require a user to execute and prove. In some cases a smart contract should be able to execute when certain conditions are met or simply when it is called. Network transactions ensure liveness of smart contracts.
2. For public shared state of smart contracts. Network transactions allow orchestrated state changes of public smart contracts without race conditions. See <here> for an in-depth explanation of public share state on Miden.
3. Clients may not have sufficient resources to generate zk-proofs.

![Local vs network transactions](../img/architecture/transaction/local-vs-network-transaction.png)

The ability to facilitate both, local and network transactions, is what differentiates Miden from other blockchains. Local transaction execution and proving can happen in parallel as for most transactions there is no need for public state changes. This increases the network's throughput tremendously and provides privacy. Network transactions on the other hand enable autonomous smart contracts and public shared state.

## Transaction lifecycle

Every `Transaction` describes the process of an account changing its state. This process described as a Miden-VM program resulting in a zero-knowledge proof of correct execution.

![Transaction execution process](../img/architecture/transaction/transaction-execution-process.png)

### Prerequisites
To execute a transaction, the executor must have complete knowledge of the account state and the notes used as transaction inputs. Specifically:

- `Notes`: A transaction can only consume notes if the full note data is known. For private `Notes`, the data can not be fetched from the blockchain and must be received otherwise.
- Foreign account data: Any foreign account data accessed during a transaction, whether private or public, must be available beforehand. There is no need to know the full account storage, but the data necessary for the transaction, e.g., the key/value pair that is read and the corresponding storage state root.
- Blockchain state: The current `BlockHeader` and `ChainMMR`—used to authenticate input notes—must be retrieved from the Miden operator before execution.

> **Info**
> - Usually, `Notes` that are consumed in a transaction must be recorded on-chain in order for the transaction to succeed. However, in Miden there is the concept of ephemeral notes that can be consumed in a transaction before registered on-chain. <link to notes chapter ephemeral notes> This allows for the executor to consume notes before they reach the blockchain which is useful for sub-second orders.
> - Usually, `Notes` that are consumed in a transaction must be recorded on-chain in order for the transaction to succeed. However, in Miden there is the concept of ephemeral notes that can be consumed in a transaction before registered on-chain. <link to notes chapter ephemeral notes> This allows for the executor to consume notes before they reach the blockchain which is useful for sub-second orders.
> - There is no nullifier-check during a transaction. Nullifiers are checked by the Miden operator during transaction verification. So at the transaction level, there is "double spending." If a note was already spent, i.e. there exists a nullifier for that note, the whole transaction will fail when submitted to the network.

### Transaction execution flow

![Transaction execution process](../img/architecture/transaction/transaction-program.png)

1. **Prologue**: On-chain commitments are validated against provided data.
2. **Note processing**: Input notes are executed sequentially against the account, following a defined order.
    - Notes must be consumed fully (all assets must be transferred).
    - The note script must be executed in full with the provided note inputs and `TransactionArguments`. `TransactionArguments` can be injected by the executor per `Note` at runtime.
3. **Transaction script execution (optional)**: A transaction script, if present, is executed.
    - This script can sign the transaction or directly interact with the account without using `Notes`.
4. **Epilogue**:
    - The account state is updated.
    - New `Notes` are created, transferring assets from the account to the newly created `Notes`.
    - Execution completes, resulting in an updated account state and generated proof.

## Example of consuming a `Note` in a transaction
To illustrate, consider a **basic wallet account** that exposes a `receive_asset` function. When a `Note` is consumed, its script executes against the account, calling `receive_asset`. This transfers the assets contained in the `Note` to the `Account`.

### Note spent conditions
`Note` creators often impose conditions on who can consume a `Note`. These restrictions are enforced by the note script, which must be fully executed by the consuming `Account`. For instance:

- A **P2ID** note verifies that the executing account's ID matches the expected account Id from the note inputs.
- A **Swap** note allows asset exchange based on predefined conditions. Example:
    - The `Note`'s spent condition is defined as "anyone can consume this note to take X units of Asset A if they simultaneously create a note sending Y units of Asset B back to the sender."
    - If an executor wants to buy only a fraction `(X-m)` of Asset A, they provide this amount via `TransactionArguments`.
    - The note script then enforces the correct transfer:
        - A new note is created returning `Y-((m*Y)/X)` of Asset B to the sender.
        - A second note is created, holding the remaining X-m of Asset A for future consumption.

### Account Interaction via Transaction Script
Not all transactions require notes. For example, the owner of a faucet can mint new tokens using only a transaction script, without interacting with external notes.

If the transaction succeeds, a proof is generated of the correct transaction execution. This proof together with the corresponding data needed for verification and updates on the global state can then be submitted and processed by the network.

> **Info**
> - One of the main reasons for separating out the execution and proving steps is to allow _stateless provers_; i.e., the executed transaction has all the data it needs to re-execute and prove a transaction without database access. This supports easier proof-generation distribution.

More notable transaction features:

- It is possible to set transaction expiration heights and in doing so, to define a block height until a transaction should be included into a block. If the transaction is expired, the resulting account state change is not valid and the transaction can not be verified anymore.
- Note and transaction scripts can read the state of foreign accounts during execution. This is called foreign procedure invocation. For example the price of an asset for the SWAP script might depend on a certain value stored in the oracle account.
