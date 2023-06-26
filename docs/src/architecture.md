# Architecture
The Polygon Miden Architecture decribes the concepts of how the participants of the network can interact. 

The architecture reflects the design goals for the rollup: 

* **High throughput** 
* **Privacy**
* **Asset Safety**

On Miden, developers can build dApps currently infeasible anywhere else. 

## Actor Model 
The [Actor Model](https://en.wikipedia.org/wiki/Actor_model) inspires Miden to achieve concurrent and local state changes. Actors are little state machines with inboxes, meaning each actor is responsible for their state. Actors can send and receive messages to communicate with other actors. Messages can be read asynchronously.

## Concepts in Miden
There are accounts and notes which can hold assets. Accounts consume and produce notes in transactions. Transactions are account state changes of single accounts. The state model captures all individual states of all accounts and notes. Finally, the execution model describes state progress in a sequence of blocks. 

### Accounts
Accounts can hold assets and define rules how those can be transferred. Accounts can represent users or autonomous smart contracts. This chapter describes the design, the storage types, and the creation of an account.

### Notes
Notes are messages carrying assets that accounts can send to each other. A note stores assets and a script that defines how this note can be consumed. This chapter describes the design, the storage types, and the creation of a note.

### Assets
Assets can be fungible and non-fungible. They are stored in the owner’s account itself or in a note. This chapter describes asset issuance, customization, and storage.

### Transactions
Transactions describe production or consumption of notes by a single account. For every transaction there is always a STARK proof in Miden. This chapter describes the transaction design and the different transaction modes.

### State Model
State describes everything that is the case at a certain point in time. Individual state of accounts or notes can be stored onchain and offchain to provide privacy. This chapter describes the three different state databases in Miden. 

### Execution Model
Execution describes how the state progresses - on an individual level via transactions and at the global level expressed as aggregated state updates in blocks. This chapter describes the execution model and how blocks are built.
