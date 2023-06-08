# Architecture
The Polygon Miden Architecture decribes the concepts of how the participants of the network can interact with each other. The architecture reflects the design goal: We aim to build an Ethereum scaling solution that extends its feature set. We want to enable developers to build faster, safer and more private dApps. Rollups allow the creation of new design spaces while retaining the collateral security of Ethereum. This is where to innovate, whereas the base layer should provide stability and evolve slowly.

The [Actor Model](https://en.wikipedia.org/wiki/Actor_model) is our inspiration for achieving concurrent, local state changes in a distributed system like a blockchain. In the actor model, actors play the role of little state machines, meaning each actor is responsible for their own state. Actors have inboxes to send and receive messages to communicate with other actors. Messages can be read asynchronously.

## Concepts in Miden
Users can interact on the network executing transactions. In Polygon Miden, there are accounts and notes, both of which can hold assets. Transactions can be understood as facilitating account state changes. Basically, they take one single account and some notes as input and output the same account at a new state together with some other notes.

The concepts which constitutes the Miden Architecture are 

* Accounts and notes and the transaction model
* State and execution model

## Transaction life cycle 
To illustrate the core protocol, let's look at how Alice can send Bob 5 MATIC in Polygon Miden. The core concepts of Polygon Miden are Accounts, Notes and Transactions. 

_Note: Because of the asynchronous execution model two transactions are needed to transfer the assets._

<p align="center">
    <img src="./diagrams/architecture/transaction_lifecycle/Account_Alice_1.png">
</p>

Alice owns an account that holds her assets.

<p align="center">
    <img src="./diagrams/architecture/transaction_lifecycle/Transaction_1.png">
</p>

Alice can execute a transaction that creates a note carrying 5 MATIC and changing her account to own 5 MATIC less.

<p align="center">
    <img src="./diagrams/architecture/transaction_lifecycle/Account_Note_Account.png">
</p>

Now in Miden there would be Alice's account, the note, and Bob's account. Because Bob hasn't consumed the note yet.

<p align="center">
    <img src="./diagrams/architecture/transaction_lifecycle/Transaction_2.png">
</p>

For Bob to finally receive the 5 MATIC, he needs to consume the note that Alice created in her transaction. To do so, Bob needs to execute a second transaction.

<p align="center">
    <img src="./diagrams/architecture/transaction_lifecycle/Account_Bob_1.png">
</p>

Now, Bob got 5 MATIC in his account. 

## State and Execution Model
The state model defines how the current state of all accounts and notes at a certain point in time can be thought of. And the execution model defines the rules about how this state progresses from `t` to `t+1`.
