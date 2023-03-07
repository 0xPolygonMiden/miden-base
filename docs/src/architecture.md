# Architecture
The Polygon Miden Architecture decribes the concepts of how the participants of the network can interact with each other. The Architecture reflects the design goal: We aim to build an Ethereum scaling solution that extends its feature set. Rollups allow the creation of new design spaces while retaining the collateral security of Ethereum. This is where to innovate, whereas the base layer should provide stability and evolve slowly.

We believe that to attract billions of users to Ethereum, we need to extend it on three dimensions: scalability, safety, and privacy

## Scalability

## Safety

## Privacy

Users can interact on the network executing transactions. Transactions are always executed against single accounts and produce or consume at least one note. The state of all accounts and notes is compressed into the L2 state root which is being published and accompanied by a ZK proof and additional data to Ethereum.

The actor model is our inspiration for achieving concurrent, local state changes on a distributed system like a blockchain. In the actor model, actors play the role of little state machines, meaning each actor is responsible for their own state. Actors have inboxes to send and receive messages to communicate with other actors. Messages can be read asynchronously.

The concepts which constitutes the Miden architecture are 

* Accounts 
* Notes
* Transactions Model
* State Model
* Execution Model

## Transaction life cycle 
To illustrate the core protocol, let's look at how Alice can send Bob 5 MATIC in Polygon Miden. Note: Because of the asynchronous execution model two transactions are needed to transfer the assets.

Alice owns an account that holds her assets.


<p align="center">
    <img src="./diagrams/architecture/transaction_lifecycle/Account_Alice_1.svg">
</p>

Alice can execute a transaction that creates a note carrying 5 MATIC and changing her account to own 5 MATIC less.

<p align="center">
    <img src="./diagrams/architecture/transaction_lifecycle/Transaction_1.svg">
</p>

Now in Miden there would be Alice's account, the note, and Bob's account. Because Bob hasn't consumed the note yet.

<p align="center">
    <img src="./diagrams/architecture/transaction_lifecycle/Account_Note_Account.svg">
</p>

For Bob to finally receive the 5 MATIC, he needs to consume the note that Alice created in her transaction. To do so, Bob needs to execute a second transaction.

<p align="center">
    <img src="./diagrams/architecture/transaction_lifecycle/Transaction_2.svg">
</p>

Now, Bob got 5 MATIC in his account. 

<p align="center">
    <img src="./diagrams/architecture/transaction_lifecycle/Account_Bob_1.svg">
</p>

## State and Execution
The State Model defines how the current state of all accounts and notes at a certain point in time can be thought of. And the Execution Model defines the rules about how this state progresses from `t` to `t+1`.
