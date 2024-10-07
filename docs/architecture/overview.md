---
comments: true
---

# Miden architecture overview

Polygon Miden’s architecture departs considerably from typical blockchain designs to support privacy and parallel transaction execution.

In traditional blockchains, state and transactions must be transparent to be verifiable. This is necessary for block production and execution.

However, user generated zero-knowledge proofs allow state transitions, e.g. transactions, to be verifiable without being transparent.

## Miden design goals

* High throughput: The ability to process a high number of transactions (state changes) over a given time interval.
* Privacy: The ability to keep data known to one’s self and anonymous while processing and/or storing it.
* Asset safety: Maintaining a low risk of mistakes or malicious behavior leading to asset loss.

## Actor model

The [actor model](https://en.wikipedia.org/wiki/Actor_model) inspires Polygon Miden’s execution model. This is a well-known computational design paradigm in concurrent systems. In the actor model, actors are state machines responsible for maintaining their own state. In the context of Polygon Miden, each account is an actor. Actors communicate with each other by exchanging messages asynchronously. One actor can send a message to another, but it is up to the recipient to apply the requested change to their state.

Polygon Miden’s architecture takes the actor model further and combines it with zero-knowledge proofs. Now, actors not only maintain and update their own state, but they can also prove the validity of their own state transitions to the rest of the network. This ability to independently prove state transitions enables local smart contract execution, private smart contracts, and much more. And it is quite unique in the rollup space. Normally only centralized entities - sequencer or prover - create zero-knowledge proofs, not the users.

## Core concepts

Miden uses _accounts_ and _notes_, both of which hold assets. Accounts consume and produce notes during transactions. Transactions describe the account state changes of single accounts.

### Accounts

[Accounts](accounts.md) can hold assets and define rules how assets can be transferred. Accounts can represent users or autonomous smart contracts. The [accounts chapter](accounts.md) describes the design of an account, its storage types, and creating an account.

### Notes

[Notes](notes.md) are messages that accounts send to each other. A note stores assets and a script that defines how the note can be consumed. The [note chapter](notes.md) describes the design, the storage types, and the creation of a note.

### Assets

[Assets](assets.md) can be fungible and non-fungible. They are stored in the owner’s account itself or in a note. The [assets chapter](assets.md) describes asset issuance, customization, and storage.

### Transactions

[Transactions](transactions/overview.md) describe the production and consumption of notes by a single account. 

Executing a transaction always results in a STARK proof. 

The [transaction chapter](transactions/overview.md) describes the transaction design and implementation, including an in-depth discussion of how transaction execution happens in the transaction kernel program.

### Limits
[Limits](limits.md) topic describes limits currently enforced in `miden-base` and `miden-node`. 

##### Accounts produce and consume notes to communicate

![Architecture core concepts](../img/architecture/miden-architecture-core-concepts.gif)

## State and execution

The actor-based execution model requires a radically different approach to recording the system's state. Actors and the messages they exchange must be treated as first-class citizens. Polygon Miden addresses this by combining the state models of account-based systems like Ethereum and UTXO-based systems like Bitcoin and Zcash.

Miden's state model captures the individual states of all accounts and notes, and the execution model describes state progress in a sequence of blocks.

### State model
[State](state.md) describes everything that is the case at a certain point in time. Individual states of accounts or notes can be stored on-chain and off-chain. This chapter describes the three different state databases in Miden.

### Execution model

[Execution](execution.md) defines how state progresses as aggregated-state-updates in batches, blocks, and epochs. The [execution chapter](execution.md) describes the execution model and how blocks are built.

##### Operators capture and progress state

![Architecture state process](../img/architecture/miden-architecture-state-progress.gif)