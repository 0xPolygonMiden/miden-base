# Architecture
The Polygon Miden Architecture decribes the concepts of how the participants of the network can interact.

The architecture reflects the design goals for the rollup:

* **High throughput**
* **Privacy**
* **Asset safety**

On Miden, developers can build dApps currently infeasible anywhere else.

## Inspired by the Actor Model
The [Actor Model](https://en.wikipedia.org/wiki/Actor_model) inspired Miden to achieve concurrent and local state changes. In the model, actors are little state machines with inboxes, meaning each actor is responsible for their own state. Actors can send and receive messages to communicate with other actors. Messages can be read asynchronously.

## Concepts in Miden
In Miden, there are accounts and notes which can hold assets. Accounts consume and produce notes in transactions. Transactions are account state changes of single accounts. The state model captures all individual states of all accounts and notes. Finally, the execution model describes state progress in a sequence of blocks.

### Accounts
Accounts can hold assets and define rules how assets can be transferred. Accounts can represent users or autonomous smart contracts. This chapter describes the design, the storage types, and the creation of an account.

### Notes
Notes are messages that accounts send to each other. A note stores assets and a script that defines how this note can be consumed. This chapter describes the design, the storage types, and the creation of a note.

### Assets
Assets can be fungible and non-fungible. They are stored in the owner’s account itself or in a note. This chapter describes asset issuance, customization, and storage.

### Transactions
Transactions describe production and consumption of notes by a single account. Every transaction results always in a STARK proof. This chapter describes the transaction design and the different transaction modes.

### State model
State describes everything that is the case at a certain point in time. Individual states of accounts or notes can be stored onchain and offchain. This chapter describes the three different state databases in Miden.

### Execution model
Execution describes how the state progresses - on an individual level via transactions and at the global level expressed as aggregated state updates in blocks. This chapter describes the execution model and how blocks are being built.
