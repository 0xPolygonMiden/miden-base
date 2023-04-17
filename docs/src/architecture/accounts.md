# Accounts
An account is an entity which holds assets and defines rules of how these assets can be transferred. The diagram below illustrates basic components of an account. In Miden every Account is a smart contract.

<p align="center">
    <img src="../diagrams/architecture/account/Account_Definition.png">
</p>

In the above:

* **Account ID** is a unique identifier (8 bytes) of an account which does not change throughout its lifetime. It also specifies the type of the underlying account. 
* **Storage** is user-defined data which can be stored in an account. The data is stored as key-value map which maps 256-bit keys to 256-bit values.
* **Nonce** is the account's nonce increments every time the account changes its state. 
* **Vault** is collection of assets stored in an account. In this collection all fungible assets with the same ID are grouped together, and all non-fungible assets are represented as individual entries.
* **Code** is a collection of functions which define an external interface for an account. The internal state of the account can only be modified by invoking these functions.

Functions exposed by the account have the following properties:

* Functions are actually roots of Miden program MASTs (i.e., 32-byte hash). Thus, function identifier is a commitment to the code which is executed when a function is invoked.
* Only account functions have mutable access to an account's storage and vault. Said another way, the only way to modify an account's internal state is through one of account's functions.
* Account functions can take parameters and can create new notes.

## Types of Accounts
There are four types of Accounts in Miden. The first two bits of the Account ID specify the type of the account.

### Regular Account with updatable code
This account type will be used by most users. They can specify and change their code and use this account for a wallet. The Account ID will start with `00`.

### Regular Account with immutable code
This account type will be used by most regular smart contracts. Once deployed the code should not change and no one should be able to change it. The Account ID will start with `01`.

### Fungible asset faucet with immutable code
Assets need to be created by Accounts in Miden. The Account ID will start with `10`. 

### Non-fungible asset faucet with immutable code
Assets need to be created by Accounts in Miden. The Account ID will start with `11`. 


## Account data storage
Account data can be public or private.

* Accounts with public state, where the actual state is stored on chain. These would be similar to how accounts work in public blockchains.
* Accounts with private state, where only the hash of the account is stored on chain. The hash is defined as `hash([account ID], [storage root], [vault root], [code root])`.

The third most significant bit of the ID specifies whether the account data is public (0) or private (1).

Missing: 

* How are accounts being created?
* Life cycle of an account?
* Account Wallet interface ?
* Account IDs
