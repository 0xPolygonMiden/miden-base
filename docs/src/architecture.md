# Architecture
The Miden Architecture document describes the concept of how the participants of the network interact with each other in Polygon Miden. The architecture reflects our design goal of **building an Ethereum-scaling solution that extends its feature set**. Miden enables developers to build faster, safer, and more private decentralized applications. Rollups allow the creation of new design spaces while retaining the collateral economic security of Ethereum. This is where we innovate while the base layer provides stability and keeps evolving slowly.

The [**Actor Model**](https://en.wikipedia.org/wiki/Actor_model) is our inspiration for achieving concurrent, local state changes in distributed systems like a blockchain. In the Actor Model, actors play the role of little state machines, meaning each actor is responsible for their own state. Actors have inboxes to send and receive messages to communicate with other actors. Messages can be read asynchronously.

## Basic Concepts
Users can interact with the network by executing transactions. There are **Accounts** and **Notes** in Polygon Miden, both of which can hold assets. Transactions can be thought of as facilitating changes to account states. They basically take one single account and certain notes as input and output the same account in a different condition along with some other notes.

The Miden architecture's core concepts are as follows: 

* Accounts, Notes and Transaction model
* State and Execution model

## Transaction Life Cycle
Let's examine how Alice can send Bob 5 MATIC in Polygon Miden to demonstrate the primary protocol. We'll be showcasing all three basic concepts i.e., Accounts, Notes, and Transactions.

_Note: Due to the asynchronous execution model, the assets must be transferred in two separate transactions._

Let's assume that there exist two accounts in Miden that belong to Alice and Bob respectively.

- Alice owns an **Account** that holds her assets.

    <p align="center">
        <img src="./diagrams/architecture/transaction_lifecycle/Account_Alice_1.png">
    </p>
    
- Now, Alice executes a **Transaction** that creates a **Note** carrying 5 MATIC and changing her **Account** balance by 5 MATIC.

    <p align="center">
        <img src="./diagrams/architecture/transaction_lifecycle/Transaction_1.png">
    </p>

- In Miden, there would be Alice's account, the note and Bob's account (becuase Bob hasn't consumed the note yet).

    <p align="center">
        <img src="./diagrams/architecture/transaction_lifecycle/Account_Note_Account.png">
    </p>

- For Bob to finally receive the 5 MATIC, he needs to consume the note that Alice created in her transaction. To do that, Bob needs to execute a second transaction.

    <p align="center">
        <img src="./diagrams/architecture/transaction_lifecycle/Transaction_2.png">
    </p>

- Now, Bob gets the 5 MATIC in his account. Voila! the transfer of assets is completed.

    <p align="center">
        <img src="./diagrams/architecture/transaction_lifecycle/Account_Bob_1.png">
    </p>

## State and Execution Model
**State model** defines how the current state of all accounts and notes at a certain point in time can be thought of. And the **Execution model** defines the rules about how this state progresses from `t` to `t+1`.

In the upcoming sections of this documentation, we'll dive deeper into State and Exectuions as well.
