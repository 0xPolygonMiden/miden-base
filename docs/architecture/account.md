# Account

An `Account` represents the primary entity of the protocol. Capable of holding assets, storing data, and executing custom code. Each `Account` is a specialized smart contract providing a programmable interface for interacting with its state and assets.

## What is the purpose of an account?

In Miden's hybrid UTXO and account-based model `Account`s enable the creation of expressive smart contracts via a Turing-complete language.

## Account core components

An `Account` is composed of several core components, illustrated below:

<p style="text-align: center;">
    <img src="../img/architecture/account/account-definition.png" style="width:30%;" alt="Account diagram"/>
</p>

These components are:

1. [ID](#id)
2. [Code](#code)
3. [Storage](#storage)
4. [Vault](#vault)
5. [Nonce](#nonce)  

### ID

> An immutable and unique identifier for the `Account`.

A 63-bit long number represents the `Account` ID. It's four most significant bits encode:
- [**Account type:**](#the-accounts-type) basic or faucet.  
- [**Account storage mode:**](#the-accounts-storage-mode) public or private.

This encoding allows the ID to convey both the `Account`’s unique identity and its operational settings.

### Code

> A collection of functions defining the `Account`’s programmable interface.

Every Miden `Account` is essentially a smart contract. The `Code` component defines the account’s functions, which can be invoked through both [Note scripts](notes.md#the-note-script) and transaction scripts. Key characteristics include:

- **Mutable access:** Only the `Account`’s own functions can modify its storage and vault. All state changes—such as updating storage slots, incrementing the nonce, or transferring assets—must occur through these functions.  
- **Function commitment:** Each function can be called by its [MAST](https://0xpolygonmiden.github.io/miden-vm/user_docs/assembly/main.html) root. The root represents the underlying code tree as a 32-byte hash. This ensures integrity, i.e., the caller calls what he expects.
- **Note creation:** `Account` functions can generate new notes.

### Storage

> A flexible, arbitrary data store within the `Account`.

The [storage](../../objects/src/accounts/storage/mod.rs) is divided into a maximum of 255 indexed [storage slots](../../objects/src/accounts/storage/slot/mod.rs). Each slot can either store a 32-byte value or serve as a pointer to a key-value store with large amounts capacity.

- **`StorageSlot::Value`:** Contains 32 bytes of arbitrary data.  
- **`StorageSlot::Map`:** Contains a [StorageMap](../../objects/src/accounts/storage/map.rs), a key-value store where both keys and values are 32 bytes. The slot's value is a commitment (hash) to the entire map.

### Vault

> A collection of [assets](assets.md) stored by the `Account`.

Large amounts of fungible and non-fungible assets can be stored in the `Account`s vault.

### Nonce

> A counter incremented with each state update to the `Account`.

The nonce enforces ordering and prevents replay attacks. It must strictly increase with every `Account` state update. The increment must be less than `2^32` but always greater than the previous nonce, ensuring a well-defined sequence of state changes.

## Account lifecycle

Throughout its lifetime, an `Account` progresses through various phases:

- **Creation and Deployment:** Initialization of the `Account` on the network.  
- **Active Operation:** Continuous state updates via `Account` functions that modify the storage, nonce, and vault.  
- **Termination or Deactivation:** Optional, depending on the contract’s design and governance model.

### Account creation

For an `Account` to be recognized by the network, it must exist in the [account database](state.md#account-database) maintained by Miden node(s).

However, a user can locally create a new `Account` ID before it’s recognized network-wide. The typical process might be:

1. Alice generates a new `Account` ID locally (according to the desired `Account` type) using the Miden client.  
2. The Miden client checks with a Miden node to ensure the ID does not already exist.  
3. Alice shares the new ID with Bob (for example, to receive assets).  
4. Bob executes a transaction, creating a note containing assets for Alice.  
5. Alice consumes Bob’s note in her own transaction to claim the asset.  
6. Depending on the `Account`’s storage mode and transaction type, the operator receives the new `Account` ID and, if all conditions are met, includes it in the `Account` database.

## Additional information

### Account type

There are two main categories of `Account`s in Miden: **basic accounts** and **faucets**.

- **Basic Accounts:**  
  Basic Accounts may be either mutable or immutable:
  - *Mutable:* Code can be changed after deployment.
  - *Immutable:* Code cannot be changed once deployed.

- **Faucets:**  
  Faucets are always immutable and can be specialized by the type of assets they issue:
  - *Fungible Faucet:* Can issue fungible [assets](assets.md).
  - *Non-fungible Faucet:* Can issue non-fungible [assets](assets.md).

Type and mutability are encoded in the two most significant bits of the `Account`'s [ID](#id).

### Account storage mode

Users can choose whether their `Account`s are stored publicly or privately. The preference is encoded in the third and forth most significant bits of the `Account`s [ID](#id):

- **Public `Account`s:**  
  The `Account`’s state is stored on-chain, similar to how `Account`s are stored in public blockchains like Ethereum. Contracts that rely on a shared, publicly accessible state (e.g., a DEX) should be public.

- **Private `Account`s:**  
  Only a commitment (hash) to the `Account`’s state is stored on-chain. This mode is suitable for users who prioritize privacy or plan to store a large amount of data in their `Account`. To interact with a private `Account`, a user must have knowledge of its interface.

The storage mode is chosen during `Account` creation, it cannot be changed later.
