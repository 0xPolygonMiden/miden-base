# Notes

> The medium through which [Accounts](accounts.md) communicate in the Miden protocol

## What is the purpose of a note?

In Miden's hybrid UTXO and account-based model notes represent UTXO's which enable parallel transaction execution and privacy through asynchronous local note production and consumption. 

## What is a note?

A note in Miden holds assets and defines how these assets can be consumed.

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

> An [asset](assets.md) container for a note.

A note can contain up to `256` different assets. These assets represent fungible or non-fungible tokens, enabling flexible asset transfers.

### Script

> The code executed when the note is consumed.

Each note has a script that defines the conditions under which it can be consumed. When accounts consume notes in transactions, note scripts call the account’s interface functions. This enables all sorts of operations beyond simple asset transfers. The Miden VM’s Turing completeness allows for arbitrary logic, making note scripts highly versatile.

### Inputs

> Arguments passed to the note script during execution.

A note can have up to `128` input values, which adds up to a maximum of 1 KB of data. The note script can access these inputs. They can convey arbitrary parameters for note consumption. 

### Serial number

> A unique and immutable identifier for the note.

The serial number helps prevent linkability between the note’s hash and its nullifier. It should be a random `Word` chosen by the user. If leaked, the note’s nullifier can be easily computed, potentially compromising privacy.

### Metadata

> Additional note information.

Notes include metadata such as the sender’s account ID and a [tag](#note-discovery) that aids in discovery. Regardless of [storage mode](#note-storage-mode), these metadata fields remain public.

## Note Lifecycle

![Architecture core concepts](../img/architecture/note/note-life-cycle.png)

The note lifecycle proceeds through four primary phases: **creation**, **validation**, **discovery**, and **consumption**. Throughout this process, notes function as secure, privacy-preserving vehicles for asset transfers and logic execution.

### Note creation

Accounts can create notes in a transaction. The note exists if it is included in the global Notes DB.

- **Users:** Executing local or network transactions.
- **Miden operators:** Facilitating on-chain actions, e.g. such as executing user notes against a DEX or other contracts.

#### Note storage mode

As with [accounts](accounts.md), notes can be stored either publicly or privately:

- **Public mode:** The note data is stored in the [note database](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#notes-database), making it fully visible on-chain.
- **Private mode:** Only the note’s hash is stored. The note’s actual data remains off-chain, enhancing privacy.

#### Ephemeral note

These use-case specific notes can be consumed even if not yet validated by being chained together into one final proof. This can allow for example sub second communication below blocktimes by adding additional trust assumptions.

### Note validation

Once created, a note must be validated by a Miden operator. Validation involves checking the transaction proof that produced the note to ensure it meets all protocol requirements.

- **Private Notes:** Only the note’s hash is recorded on-chain, keeping the data confidential.
- **Public Notes:** The full note data is stored, providing transparency for applications requiring public state visibility.

After validation, notes become “live” and eligible for discovery and eventual consumption.

### Note discovery

Clients often need to find specific notes of interest. Miden allows clients to query the note database using note tags. These lightweight, 32-bit tags serve as best-effort filters, enabling quick lookups for notes related to particular use cases, scripts, or account prefixes.

Using note tags strikes a balance between privacy and efficiency. Without tags, querying a specific note ID reveals a user’s interest to the operator. Conversely, downloading and filtering all registered notes locally is highly inefficient. Tags allow users to adjust their level of privacy by choosing how broadly or narrowly they define their search criteria, letting them find the right balance between revealing too much information and incurring excessive computational overhead.

### Note consumption

To consume a note, the consumer must know its data, including the inputs needed to compute the nullifier. Consumption occurs as part of a transaction. Upon successful consumption a nullifier is generated for the consumed notes.

Upon successful verification of the transaction:

1. The Miden operator records the note’s nullifier as “consumed” in the nullifier database.
2. The note’s one-time claim is thus extinguished, preventing reuse.

#### Note recipient - restricting consumption

Consumption of a note can be restricted to certain accounts or entities. For instance, the P2ID and P2IDR note scripts target a specific account ID. Alternatively, Miden defines a `RECIPIENT` (represented as a `Word`) computed as:

```arduino
hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash)
```

Only those who know the `RECIPIENT`’s pre-image can consume the note. For private notes, this ensures an additional layer of control and privacy, as only parties with the correct data can claim the note.

The [transaction prologue](transactions/kernel.md) requires all necessary data to compute the note hash. This setup allows scenario-specific restrictions on who may consume a note.

For a practical example, refer to the [SWAP note script](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/note_scripts/SWAP.masm), where the `RECIPIENT` ensures that only a defined target can consume the swapped asset.

#### Note nullifier - ensuring private consumption

The note nullifier, computed as:

```arduino
hash(serial_num, script_hash, input_hash, vault_hash)
```

This achieves the following properties:

- Every note can be reduced to a single unique nullifier.
- One cannot derive a note's hash from its nullifier.
- To compute the nullifier, one must know all components of the note: `serial_num`, `script_hash`, `input_hash`, and `vault_hash`.

That means if a note is private and the operator stores only the note's hash, only those with the note details know if this note has been consumed already. Zcash first [introduced](https://zcash.github.io/orchard/design/nullifiers.html#nullifiers) this approach.

![Architecture core concepts](../img/architecture/note/nullifier.png)

## Conclusion

Miden’s notes introduce a powerful mechanism for secure, flexible, and private state management. By enabling asynchronous asset transfers, parallel execution, and privacy at scale, they transcend the limitations of strictly account-based models. As a result, developers and users alike enjoy enhanced scalability, confidentiality, and control. With these capabilities, Miden is paving the way for true **programmable money** where assets, logic, and trust converge seamlessly.
