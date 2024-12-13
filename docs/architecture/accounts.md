# Accounts

> The primary entities of the Miden protocol

## What is an account?

In Miden, an `Account` represents an entity capable of holding assets, storing data, and executing custom code. Each `Account` is essentially a specialized smart contract providing a programmable interface for interacting with its state and managed assets.

### Account type

There are two main categories of accounts in Miden: **basic accounts** and **faucets**.

- **Basic Accounts:**  
  Basic accounts may be either mutable or immutable:
  - *Mutable:* Code can be changed after deployment.
  - *Immutable:* Code cannot be changed once deployed.

- **Faucets:**  
  Faucets are always immutable and can be specialized by the type of assets they issue:
  - *Fungible Faucet:* Can issue fungible [assets](assets.md).
  - *Non-fungible Faucet:* Can issue non-fungible [assets](assets.md).

Type and mutability are encoded in the two most significant bits of the account's ID:

|                        | Basic Mutable | Basic Immutable | Fungible Faucet | Non-fungible Faucet |
|------------------------|---------------|-----------------|-----------------|---------------------|
| **Description**        | For general user accounts (e.g., a wallet) where code changes are allowed. | For typical smart contracts where code is fixed after deployment. | Allows to issue and manage fungible assets. | Allows to issue and manage non-fungible assets. |
| **Code updatability**  | Yes           | No              | No              | No                  |
| **Most significant bits** | `00`        | `01`            | `10`            | `11`                |

### Account storage mode

Users can choose whether their accounts are stored publicly or privately. The third and fourth most significant bits of the [ID](#id) encode this preference:

- **Public Accounts:**  
  The account’s state is stored on-chain, similar to how accounts are stored in public blockchains like Ethereum. Contracts that rely on a shared, publicly accessible state (e.g., a DEX) should be public.

- **Private Accounts:**  
  Only a commitment (hash) to the account’s state is stored on-chain. This mode is suitable for users who prioritize privacy and off-chain data management. To interact with a private account, a user must have knowledge of its interface.

## Account core components

An `Account` is composed of several core components, illustrated below:

<p style="text-align: center;">
    <img src="../img/architecture/account/account-definition.png" style="width:30%;" alt="Account diagram"/>
</p>

These components are:

1. [ID](#id)  
2. [Storage](#storage)  
3. [Nonce](#nonce)  
4. [Vault](#vault)  
5. [Code](#code)

### ID

> An immutable and unique identifier for the `Account`.

The `Account` ID is a single field element (`felt`) of about 63 bits. The four most significant bits encode:  
- [**Account type:**](#the-accounts-type) basic or faucet.  
- [**Account storage mode:**](#the-accounts-storage-mode) public or private.

This encoding allows the ID to convey both the account’s unique identity and its operational settings.

### Storage

> A flexible, arbitrary data store within the `Account`.

The [storage](../../objects/src/accounts/storage/mod.rs) of an `Account` consists of up to 255 indexed [storage slots](../../objects/src/accounts/storage/slot/mod.rs). Each slot can be one of the following types:

- **`StorageSlot::Value`:** Contains a single `Word` (32 bytes) of arbitrary data.  
- **`StorageSlot::Map`:** Contains a [StorageMap](../../objects/src/accounts/storage/map.rs), a key-value store where both keys and values are `Word`s. The slot's value is a commitment (hash) to the entire map.

### Nonce

> A counter incremented with each state update to the `Account`.

The `nonce` enforces ordering and prevents replay attacks or double-spending. It must strictly increase with every account state update. The increment must be less than `2^32` but always greater than the previous nonce, ensuring a well-defined sequence of state changes.

### Vault

> A collection of [assets](assets.md) stored by the `Account`.

Assets are stored in a sparse Merkle tree, allowing for efficient, cryptographically secure proofs of ownership and state:

- **Fungible assets:** Indexed by the issuing faucet ID. Each fungible asset type occupies exactly one node in the tree.  
- **Non-fungible assets (NFTs):** Indexed by a unique asset identifier. Each NFT is represented by a distinct node in the tree.

This arrangement facilitates efficient queries, updates, and proofs on assets.

### Code

> A collection of functions defining the `Account`’s programmable interface.

Every Miden account is essentially a smart contract. The `Code` component defines the account’s functions, which can be invoked through both [Note scripts](notes.md#the-note-script) and transaction scripts. Key characteristics include:

- **Function commitment:** Each function corresponds to a root of a [Miden program MAST](https://0xpolygonmiden.github.io/miden-vm/user_docs/assembly/main.html), represented as a 32-byte hash. This ensures both integrity and immutability of the underlying function logic.  
- **Mutable access:** Only the account’s own functions can modify its storage and vault. All state changes—such as updating storage slots, incrementing the nonce, or transferring assets—must occur through these functions.  
- **Note creation:** Account functions can generate new notes.

## Account lifecycle

Throughout its lifetime, an `Account` progresses through various phases:

- **Creation and Deployment:** Initialization of the account on the network.  
- **Active Operation:** Continuous state updates via account functions that modify the storage, nonce, and vault.  
- **Termination or Deactivation:** Optional, depending on the contract’s design and governance model.

### Account creation

For an account to be recognized by the network, it must exist in the [account database](state.md#account-database) maintained by Miden node(s).

However, a user can locally create a new account ID before it’s recognized network-wide. The typical process might be:

1. Alice generates a new account ID locally (according to the desired account type) using the Miden client.  
2. The Miden client checks with a Miden node to ensure the ID does not already exist.  
3. Alice shares the new ID with Bob (for example, to receive assets).  
4. Bob executes a transaction, creating a note containing assets for Alice.  
5. Alice consumes Bob’s note in her own transaction to claim the asset.  
6. Depending on the account’s storage mode and transaction type, the operator receives the new account ID and, if all conditions are met, includes it in the account database.

Users may create accounts by:

- Using the [Miden client](https://0xpolygonmiden.github.io/miden-docs/miden-client/index.html) as a wallet interface.  
- Invoking built-in functions from [miden-base](https://github.com/0xPolygonMiden/miden-base) libraries to create:
  - [Basic wallets](https://github.com/0xPolygonMiden/miden-base/blob/4e6909bbaf65e77d7fa0333e4664be81a2f65eda/miden-lib/src/accounts/wallets/mod.rs#L15)  
  - [Fungible faucets](https://github.com/0xPolygonMiden/miden-base/blob/4e6909bbaf65e77d7fa0333e4664be81a2f65eda/miden-lib/src/accounts/faucets/mod.rs#L11)

## Conclusion

In this section, we covered:

- [What is an `Account`](#what-is-an-account)  
- [Its constituent components](#the-accounts-core-components)  
- [Its lifecycle](#the-accounts-lifecycle)

With this information, you are now better equipped to understand how Miden `Accounts` operate, how they manage data and assets, and how their programmable functions enable secure and flexible interactions within the Miden protocol.
