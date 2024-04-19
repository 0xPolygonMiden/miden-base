Accounts are basic building blocks representing a user or an autonomous smart contract.

For smart contracts the go-to solution is account-based state. Miden supports expressive smart contracts via a Turing-complete language and the use of accounts.

In Miden, an account is an entity which holds assets and defines rules about how to transfer these assets. 

## Account design

In Miden every account is a smart contract. The diagram below illustrates the basic components of an account. 

<center>
![Architecture core concepts](../img/architecture/account/account-definition.png){ width="25%" }
</center>

!!! tip "Key to diagram"
    * **Account ID**: A unique identifier for an account. This does not change throughout its lifetime.
    * **Storage**: User-defined data which can be stored in an account.
    * **Nonce**: A counter which increments whenever the account state changes.
    * **Vault**: A collection of assets stored in an account.
    * **Code**: A collection of functions which define the external interface for an account.

### Account ID

A `~63` bits long identifier for the account ID (one field element `felt`). 

The four most significant bits specify the [account type](#account-types) - regular or faucet - and the [storage mode](#account-storage-modes) - public or private.

### Account data storage

A storage for user-defined data that is composed of two components.

1. A simple sparse Merkle tree of depth `8` which is index addressable. This provides the user with `256` `word` slots.

2. Users requiring additional storage can use the second component a `MerkleStore`. It allows users to store any Merkle structures they need. The root of the Merkle structure can be stored as a leaf in a simple sparse Merkle tree. When `AccountStorage` is serialized it checks if any of the leaves in the simple sparse Merkle tree are Merkle roots of other Merkle structures. If any Merkle roots are found then the Merkle structures will be persisted in the `AccountStorage`'s `MerkleStore`.

### Nonce

A counter which increments whenever the account state changes. 

Nonce values must be strictly monotonically increasing and increment by any value smaller than `2^32` for every account update.

### Vault

An asset container for an account.

An account vault can contain an unlimited number of [assets](assets.md). The assets are stored in a sparse Merkle tree as follows:

* For fungible assets, the index of a node is defined by the issuing faucet ID, and the value
  of the node is the asset itself. Thus, for any fungible asset there will be only one node
  in the tree.
* For non-fungible assets, the index is defined by the asset itself, and the asset is also
  the value of the node.

An account vault can be reduced to a single hash which is the root of the sparse Merkle tree.

### Code

The interface for accounts. In Miden every account is a smart contract. It has an interface that exposes functions that can be called by [note scripts](notes.md#the-note-script) and transaction scripts. Users cannot call those functions directly.

Functions exposed by the account have the following properties:

* Functions are actually roots of [Miden program MASTs](https://0xpolygonmiden.github.io/miden-vm/user_docs/assembly/main.html) (i.e., a `32`-byte hash). Thus, the function identifier is a commitment to the code which is executed when a function is invoked.
* Only account functions have [mutable access](transactions/contexts.md) to an account's storage and vault. Therefore, the only way to modify an account's internal state is through one of the account's functions.
* Account functions can take parameters and can create new notes.

!!! note
    Since code in Miden is expressed as MAST, every function is a commitment to the underlying code. The code cannot change unnoticed to the user because its hash would change. Behind any MAST root there can only be `256` functions.

#### Example account code

Currently, Miden provides two standard implementations for account code.

##### Basic user account

There is a standard for a basic user account. It exposes three functions via its interface.

<details>
  <summary>Basic user account code</summary>

  ```arduino
    use.miden::contracts::wallets::basic->basic_wallet
    use.miden::contracts::auth::basic

    export.basic_wallet::receive_asset
    export.basic_wallet::send_asset
    export.basic::auth_tx_rpo_falcon512
  ```
</details>

[Note scripts](notes.md#the-note-script) or [transaction scripts](transactions/kernel.md#the-transaction-script-processing) can call `receive_asset` and `send_asset`. 

Transaction scripts can also call `auth_tx_rpo_falcon512` and authenticate the transaction. 

!!! warning
    Without correct authentication, i.e. knowing the correct private key, a note cannot successfully invoke `receive_asset` or `send_asset`. 

##### Basic fungible faucet (faucet for fungible assets)

There is also a standard for a [basic fungible faucet](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/miden/contracts/faucets/basic_fungible.masm).

<details>
  <summary>Fungible faucet code</summary>

  ```arduino
  #! Distributes freshly minted fungible assets to the provided recipient.
  #!
  #! ...
  export.distribute
    # get max supply of this faucet. We assume it is stored at pos 3 of slot 1
    push.METADATA_SLOT exec.account::get_item drop drop drop
    # => [max_supply, amount, tag, note_type, RECIPIENT, ...]

    # get total issuance of this faucet so far and add amount to be minted
    exec.faucet::get_total_issuance
    # => [total_issuance, max_supply, amount, tag, note_type RECIPIENT, ...]

    # compute maximum amount that can be minted, max_mint_amount = max_supply - total_issuance
    sub
    # => [max_supply - total_issuance, amount, tag, note_type, RECIPIENT, ...]

    # check that amount =< max_supply - total_issuance, fails if otherwise
    dup.1 gte assert.err=ERR_BASIC_FUNGIBLE_MAX_SUPPLY_OVERFLOW
    # => [asset, tag, note_type, RECIPIENT, ...]

    # creating the asset
    exec.asset::create_fungible_asset
    # => [ASSET, tag, note_type, RECIPIENT, ...]

    # mint the asset; this is needed to satisfy asset preservation logic.
    exec.faucet::mint
    # => [ASSET, tag, note_type, RECIPIENT, ...]

    # create a note containing the asset
    exec.tx::create_note
    # => [note_ptr, ZERO, ZERO, ...]
  end

  #! Burns fungible assets.
  #!
  #! ...
  export.burn
      # burning the asset
      exec.faucet::burn
      # => [ASSET]

      # increments the nonce (anyone should be able to call that function)
      push.1 exec.account::incr_nonce

      # clear the stack
      padw swapw dropw
      # => [...]
  end
  ```
</details>

The contract exposes two functions `distribute` and `burn`. 

The first function `distribute` can only be called by the faucet owner, otherwise it fails. As inputs, the function expects everything that is needed to create a note containing the freshly minted asset, i.e., amount, metadata, and recipient.

The second function `burn` burns the tokens that are contained in a note and can be called by anyone.

!!! info "Difference between `burn` and `distribute`"
    The `burn` procedure exposes `exec.account::incr_nonce`, so by calling `burn` the nonce of the executing account gets increased by `1` and the transaction will pass the epilogue check. The `distribute` procedure does not expose that. That means the executing user needs to call `basic::auth_tx_rpo_falcon512` which requires the private key.*

## Account creation

For an account to exist it must be present in the [account database](state.md#account-database) kept on the Miden node(s). 

However, new accounts can be created locally by users using the Miden client. The process is as follows:

* Alice creates a new account ID (according to the account types) using the Miden client.
* Alice's Miden client asks the Miden node to check if the new ID already exists.
* Alice shares the ID with Bob (eg. when Alice wants to receive funds).
* Bob executes a transaction and creates a note that contains an asset for Alice.
* Alice consumes Bob's note to receive the asset in a transaction.
* Depending on the account storage mode (private vs. public) and transaction type (local vs. network) the operator eventually receives the new account ID and - if the transaction is correct - adds the ID to the account database.

A user can create an account in one of the following manners:

1. Use the [Miden client](https://docs.polygon.technology/miden/miden-client/) as a wallet.
2. Use the Miden base builtin functions for wallet creation: [basic wallet](https://github.com/0xPolygonMiden/miden-base/blob/4e6909bbaf65e77d7fa0333e4664be81a2f65eda/miden-lib/src/accounts/wallets/mod.rs#L15), [fungible faucet](https://github.com/0xPolygonMiden/miden-base/blob/4e6909bbaf65e77d7fa0333e4664be81a2f65eda/miden-lib/src/accounts/faucets/mod.rs#L11)

## Account types

There are two basic account types in Miden: Regular accounts and faucets. Only faucets can mint new assets. Regular accounts can be mutable or immutable, which simply means that it is possible to change the account code after creation. 

Type and mutability is encoded in the most significant bits of the account's ID. 

| | Basic mutable | Basic immutable | Fungible faucet | Non-fungible faucet |
|---|---|---|---|---|
| **Description** | For most users, e.g. a wallet. Code changes allowed, including public API. | For most smart contracts. Once deployed code is immutable. | Users can issue fungible assets and customize them. | Users can issue non-fungible assets and customize them. |
| **Code updatability** | yes | no | no | no |
| **Most significant bits** | `00` | `01` | `10` | `11` |

## Public and private accounts

Users can decide whether to keep their accounts private or public at account creation. The account ID encodes this preference on the third and fourth most significant bit.

* Accounts with public state: The actual state is stored on-chain. This is similar to how accounts work in public blockchains, like Ethereum. Smart contracts that depend on public shared state should be stored public on Miden, e.g., DEX contract.
* Accounts with private state: Only the hash of the account is stored on-chain. Users who want to stay private, and manage their own data, should choose this option. Users who want to interact with private accounts need to know the account's interface.

</br>