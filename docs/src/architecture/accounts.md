# Accounts
An account is an entity which holds assets and defines rules of how these assets can be transferred. The diagram below illustrates basic components of an account. In Miden every account is a smart contract.

<p align="center">
    <img src="../diagrams/architecture/account/Account_Definition.png">
</p>

In the above:

* **Account ID** is a unique identifier (8 bytes) of an account which does not change throughout its lifetime. It also specifies the type of the underlying account. 
* **Storage** is user-defined data which can be stored in an account. 256 values of four field elements each can be stored in a key-value map mapping four field element keys to four field element values.
* **Nonce** is a counter that increments every time the account changes its state. The counter is monotonically increasing. 
* **Vault** is collection of assets stored in an account. In this collection all fungible assets with the same ID are grouped together, and all non-fungible assets are represented as individual entries.
* **Code** is a collection of functions which define an external interface for an account. The internal state of the account can only be modified by invoking these functions.

Functions exposed by the account have the following properties:

* Functions are actually roots of [Miden program MASTs](https://wiki.polygon.technology/docs/miden/user_docs/assembly/main) (i.e., 32-byte hash). Thus, function identifier is a commitment to the code which is executed when a function is invoked.
* Only account functions have mutable access to an account's storage and vault. Said another way, the only way to modify an account's internal state is through one of account's functions.
* Account functions can take parameters and can create new notes.

## Account lifecycle
For an account to exist it must be present in the [Account DB](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#account-database) kept by the Miden Node(s). However, new accounts can be created locally by users using a wallet.

The lifcycle is as follows:
* Alice grinds a new Account ID according to the account types (see below) using a wallet
* Alice's Miden Client requests the Miden Node if new Account ID already exists
* Alice shares the new Account ID to Bob, e.g. when Alice wants to receive funds
* Bob executes a transaction and creates a note that contains an asset for Alice
* Alice consumes Bob's note to receive the asset in a transaction
* Depending on account type (private vs. public) and transaction type (local vs. network) the Operator receives new Account ID eventually and - if transaction is correct - adds the ID to the Account DB

## Types of accounts
There are four types of accounts in Miden. The first two bits of the Account ID specify the type of the account.

### Regular account with updatable code
This account type will be used by most users. They can specify and change their code and use this account for a wallet. The Account ID will start with `00`.

### Regular account with immutable code
This account type will be used by most regular smart contracts. Once deployed the code should not change and no one should be able to change it. The Account ID will start with `01`.

### Fungible asset faucet with immutable code
Assets need to be created by accounts in Miden. The Account ID will start with `10`. 

### Non-fungible asset faucet with immutable code
Assets need to be created by accounts in Miden. The Account ID will start with `11`. 


## Account data storage
Account data - stored by the Miden Node - can be public or private.

* Accounts with public state, where the actual state is stored on chain. These would be similar to how accounts work in public blockchains.
* Accounts with private state, where only the hash of the account is stored on chain. The hash is defined as: \
`hash([account ID, 0, 0, nonce], [vault root], [storage root], [code root])`

The third most significant bit of the ID specifies whether the account data is public `0` or private `1`.

<p align="center">
    <img src="../diagrams/architecture/account/Account_ID.png">
</p>
