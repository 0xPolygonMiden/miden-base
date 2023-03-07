# Accounts
An account is an entity which holds assets and defines rules of how these assets can be transferred. The diagram below illustrates basic components of an account.

<p align="center">
    <img src="../diagrams/protocol/account/Account_Definition.png">
</p>

In the above:

* ID is a unique identifier (32 bytes) of an account which does not change throughout its lifetime. This is not the same as address as accounts do not have addresses.
* Storage is user-defined data which can be stored in an account. The exact mechanism is TBD. For example, this could be a key-value map or an index-based array.
* Vault is collection of assets stored in an account. In this collection all fungible assets with the same ID are grouped together, and all non-fungible assets are represented as individual entries.
* Code is a collection of functions which define an external interface for an account.

Functions exposed by the account have the following properties:

* Functions are actually roots of Miden program MASTs (i.e., 32-byte hash). Thus, function identifier is a commitment to the code which is executed when a function is invoked.
* Only account functions have mutable access to an account's storage and vault. Said another way, the only way to modify an account's internal state is through one of account's functions.
* Account functions can take parameters and can create new notes.

## Accounts with public and private state

* Accounts with public state, where the actual state is stored on chain. These would be similar to how accounts work in public blockchains.
* Accounts with private state, where only the hash of the account is stored on chain. The hash could be defined as something like `hash([account ID], [storage root], [vault root], [code root])`.

Missing: 

* How are accounts being created?
* Life cycle of an account?
* Account Wallet interface ?
* Account IDs
