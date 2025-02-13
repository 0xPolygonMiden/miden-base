# Note

A `Note` is the medium through which [Accounts](accounts.md) communicate. A `Note` holds assets and defines how they can be consumed.

## What is the purpose of a note?

In Miden's hybrid UTXO and account-based model `Note`s represent UTXO's which enable parallel transaction execution and privacy through asynchronous local `Note` production and consumption. 

## Note core components

A `Note` is composed of several core components, illustrated below:

<p style="text-align: center;">
    <img src="../img/architecture/note/note.png" style="width:30%;" alt="Note diagram"/>
</p>

These components are:

1. [Assets](#assets)  
2. [Script](#script)  
3. [Inputs](#inputs)  
4. [Serial number](#serial-number)  
5. [Metadata](#metadata)

### Assets

> An [asset](assets.md) container for a `Note`.

A `Note` can contain up to 256 different assets. These assets represent fungible or non-fungible tokens, enabling flexible asset transfers.

### Script

> The code executed when the `Note` is consumed.

Each `Note` has a script that defines the conditions under which it can be consumed. When accounts consume `Note`s in transactions, `Note` scripts call the account’s interface functions. This enables all sorts of operations beyond simple asset transfers. The Miden VM’s Turing completeness allows for arbitrary logic, making `Note` scripts highly versatile.

### Inputs

> Arguments passed to the `Note` script during execution.

A `Note` can have up to 128 input values, which adds up to a maximum of 1 KB of data. The `Note` script can access these inputs. They can convey arbitrary parameters for `Note` consumption. 

### Serial number

> A unique and immutable identifier for the `Note`.

The serial number has two main purposes. Firstly by adding some randomness to the `Note` it ensures it's uniqueness, secondly in private `Note`s it helps prevent linkability between the `Note`'s hash and its nullifier. The serial number should be a random 32 bytes number chosen by the user. If leaked, the `Note`’s nullifier can be easily computed, potentially compromising privacy.

### Metadata

> Additional `Note` information.

`Note`s include metadata such as the sender’s account ID and a [tag](#note-discovery) that aids in discovery. Regardless of [storage mode](#note-storage-mode), these metadata fields remain public.

## Note Lifecycle

![Architecture core concepts](../img/architecture/note/note-life-cycle.png)

The `Note` lifecycle proceeds through four primary phases: **creation**, **validation**, **discovery**, and **consumption**. Throughout this process, `Note`s function as secure, privacy-preserving vehicles for asset transfers and logic execution.

### Note creation

Accounts can create `Note`s in a transaction. The `Note` exists if it is included in the global `Note`s DB.

- **Users:** Executing local or network transactions.
- **Miden operators:** Facilitating on-chain actions, e.g. such as executing user `Note`s against a DEX or other contracts.

#### Note storage mode

As with [accounts](accounts.md), `Note`s can be stored either publicly or privately:

- **Public mode:** The `Note` data is stored in the [note database](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#notes-database), making it fully visible on-chain.
- **Private mode:** Only the `Note`’s hash is stored publicly. The `Note`’s actual data remains off-chain, enhancing privacy.

#### Ephemeral note

These specific `Note`s can be consumed even if not yet registered on-chain. They can be chained together into one final proof. This can allow for example sub-second communication below blocktimes by adding additional trust assumptions.

### Note validation

Once created, a `Note` must be validated by a Miden operator. Validation involves checking the transaction proof that produced the `Note` to ensure it meets all protocol requirements.

- **Private Notes:** Only the `Note`’s hash is recorded on-chain, keeping the data confidential.
- **Public Notes:** The full `Note` data is stored, providing transparency for applications requiring public state visibility.

After validation, `Note`s become “live” and eligible for discovery and eventual consumption.

### Note discovery

Clients often need to find specific `Note`s of interest. Miden allows clients to query the `Note` database using `Note` tags. These lightweight, 32-bit data fields serve as best-effort filters, enabling quick lookups for `Note`s related to particular use cases, scripts, or account prefixes.

Using `Note` tags strikes a balance between privacy and efficiency. Without tags, querying a specific `Note` ID reveals a user’s interest to the operator. Conversely, downloading and filtering all registered `Note`s locally is highly inefficient. Tags allow users to adjust their level of privacy by choosing how broadly or narrowly they define their search criteria, letting them find the right balance between revealing too much information and incurring excessive computational overhead.

### Note consumption

To consume a `Note`, the consumer must know its data, including the inputs needed to compute the nullifier. Consumption occurs as part of a transaction. Upon successful consumption a nullifier is generated for the consumed `Note`s.

Upon successful verification of the transaction:

1. The Miden operator records the `Note`’s nullifier as “consumed” in the nullifier database.
2. The `Note`’s one-time claim is thus extinguished, preventing reuse.

#### Note recipient restricting consumption

Consumption of a `Note` can be restricted to certain accounts or entities. For instance, the P2ID and P2IDR `Note` scripts target a specific account ID. Alternatively, Miden defines a RECIPIENT (represented as 32 bytes) computed as:

```arduino
hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash)
```

Only those who know the RECIPIENT’s pre-image can consume the `Note`. For private `Note`s, this ensures an additional layer of control and privacy, as only parties with the correct data can claim the `Note`.

The [transaction prologue](transactions/kernel.md) requires all necessary data to compute the `Note` hash. This setup allows scenario-specific restrictions on who may consume a `Note`.

For a practical example, refer to the [SWAP note script](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/note_scripts/SWAP.masm), where the RECIPIENT ensures that only a defined target can consume the swapped asset.

#### Note nullifier ensuring private consumption

The `Note` nullifier, computed as:

```arduino
hash(serial_num, script_hash, input_hash, vault_hash)
```

This achieves the following properties:

- Every `Note` can be reduced to a single unique nullifier.
- One cannot derive a `Note`'s hash from its nullifier.
- To compute the nullifier, one must know all components of the `Note`: serial_num, script_hash, input_hash, and vault_hash.

That means if a `Note` is private and the operator stores only the `Note`'s hash, only those with the `Note` details know if this `Note` has been consumed already. Zcash first [introduced](https://zcash.github.io/orchard/design/nullifiers.html#nullifiers) this approach.

![Architecture core concepts](../img/architecture/note/nullifier.png)
