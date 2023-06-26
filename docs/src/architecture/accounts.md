# Accounts
An account is an entity which holds assets and defines rules of how these assets can be transferred. The diagram below illustrates basic components of an account. In Miden every account is a smart contract.

<p align="center">
    <img src="../diagrams/architecture/account/Account_Definition.png">
</p>

In the above:

* **Account ID** is a unique identifier of an account which does not change throughout its lifetime.
* **Storage** is user-defined data which can be stored in an account.
* **Nonce** is a counter which must be incremented whenever account state changes.
* **Vault** is collection of assets stored in an account.
* **Code** is a collection of functions which define an external interface for an account.

## Account ID
8 bytes (60 bit) long identifier for the account. The first three significant bits specify its type and the storage mode. There are four types of accounts in Miden that can be stored in two differnt ways:

### Regular account with updatable code
This account type will be used by most users. They can specify and change their code and use this account for a wallet. The Account ID will start with `00`.

### Regular account with immutable code
This account type will be used by most regular smart contracts. Once deployed the code should not change and no one should be able to change it. The Account ID will start with `01`.

### Fungible asset faucet with immutable code
Assets need to be created by accounts in Miden. The Account ID will start with `10`.

### Non-fungible asset faucet with immutable code
Assets need to be created by accounts in Miden. The Account ID will start with `11`.

### Account storage mode
Account data - stored by the Miden Node - can be public or private. The third most significant bit of the ID specifies whether the account data is public `0` or private `1`.

* Accounts with **public state**, where the actual state is stored on chain. These would be similar to how accounts work in public blockchains.
* Accounts with **private state**, where only the hash of the account is stored on chain. The hash is defined as: \
`hash([account ID, 0, 0, nonce], [vault root], [storage root], [code root])`

### Example account ID

<p align="center">
    <img src="../diagrams/architecture/account/Account_ID.png">
</p>

## Storage
User-defined data that can be stored in an account. Account storage is composed of two components. The first component is a simple sparse Merkle tree of depth 8 which is index addressable. This provides the user with 256 Word slots. Users that require additional storage can use the second component which is a `MerkleStore`. This will allow the user to store any Merkle structures they need. This is achieved by storing the root of the Merkle structure as a leaf in the simple sparse merkle tree. When `AccountStorage` is serialized it will check to see if any of the leafs in the simple sparse Merkle tree are Merkle roots of other Merkle structures. If any Merkle roots are found then the Merkle structures will be persisted in the `AccountStorage` `MerkleStore`.

TODO ADD STORAGE DIAGRAM

## Nonce
Counter which must be incremented whenever the account state changes. Nonce values must be strictly monotonically increasing and can be incremented by any value smaller than 2^{32} for every account update.

## Vault
Asset container for an account.

An account vault can contain an unlimited number of assets. The assets are stored in a Sparse
Merkle tree as follows:
* For fungible assets, the index of a node is defined by the issuing faucet ID, and the value
  of the node is the asset itself. Thus, for any fungible asset there will be only one node
  in the tree.
* For non-fungible assets, the index is defined by the asset itself, and the asset is also
  the value of the node.

An account vault can be reduced to a single hash which is the root of the Sparse Merkle tree.

## Code
Interface for accounts. In Miden every account is a smart contract. It has an interface that exposes functions that can be called by note scripts. Functions exposed by the account have the following properties:

* Functions are actually roots of [Miden program MASTs](https://wiki.polygon.technology/docs/miden/user_docs/assembly/main) (i.e., 32-byte hash). Thus, function identifier is a commitment to the code which is executed when a function is invoked.
* Only account functions have mutable access to an account's storage and vault. Said another way, the only way to modify an account's internal state is through one of account's functions.
* Account functions can take parameters and can create new notes.

# Account creation
For an account to exist it must be present in the [Account DB](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#account-database) kept by the Miden Node(s). However, new accounts can be created locally by users using a wallet.

The process is as follows:

* Alice grinds a new Account ID according to the account types (see below) using a wallet
* Alice's Miden Client requests the Miden Node if new Account ID already exists
* Alice shares the new Account ID to Bob, e.g. when Alice wants to receive funds
* Bob executes a transaction and creates a note that contains an asset for Alice
* Alice consumes Bob's note to receive the asset in a transaction
* Depending on the account storage mode (private vs. public) and transaction type (local vs. network) the Operator receives new Account ID eventually and - if transaction is correct - adds the ID to the Account DB
