# Notes

> The medium through which [Accounts](accounts.md) communicate in the Miden protocol

## What is a Note?

A note in Miden holds assets and defines how these assets can be consumed. The note model in Miden enables parallel transaction execution and privacy. 

## The Notes Core Components

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

Each note has an associated script that defines the conditions under which it can be consumed. Because the script is executed in the context of a specific account, it may invoke that account’s functions, enabling complex operations beyond simple asset transfers. The Miden VM’s Turing completeness allows for arbitrary logic, making note scripts highly versatile.

Every note script is represented as a commitment to underlying code—via a unique hash or a Miden program MAST root. This ensures that any changes to the script are detectable, preserving trust. Scripts are accompanied by inputs defined at note creation. Additionally, executors can provide optional “note args” before execution, granting flexibility in how the script is run.

### Inputs

> Arguments passed to the note script during execution.

A note can have up to `128` input values, each stored as a single field element. These inputs can be accessed by the note script via [transaction kernel procedures](./transactions/kernel.md). With the ability to carry up to ~1 KB of data, these inputs can convey arbitrary parameters for note consumption.

### Serial number

> A unique and immutable identifier for the note.

The serial number helps prevent linkability between the note’s hash and its nullifier. It should be a random `word` chosen by the user. If leaked, the note’s nullifier can be easily computed, potentially compromising privacy.

### Metadata

> Additional note information.

Notes include metadata such as the sender’s account ID and a [tag](#note-discovery) that aids in discovery. Regardless of [storage mode](#note-storage-mode), these metadata fields remain public.

## Note Lifecycle

![Architecture core concepts](../img/architecture/note/note-life-cycle.png)

The note lifecycle proceeds through four primary phases: **creation**, **validation**, **discovery**, and **consumption**. Throughout this process, notes function as secure, privacy-preserving vehicles for asset transfers and logic execution.

### Note Creation

One or more notes can be generated as `OutputNotes` when Miden transactions complete successfully. These transactions may be initiated by:

- **Users:** Executing local or network transactions.
- **Miden operators:** Facilitating on-chain actions, e.g. such as executing user notes against a DEX or other contracts.

#### Note Storage Mode

As with [accounts](accounts.md), notes can be stored either publicly or privately:

- **Public mode:** The note data is stored in the [note database](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#notes-database), making it fully visible on-chain.
- **Private mode:** Only the note’s hash (a cryptographic commitment) is stored. The note’s actual data remains off-chain, enhancing privacy.

The note’s hash can be computed as:

```arduino
hash(hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash), vault_hash)
```

#### Note Recipient - Restricting Consumption

Consumption of a note can be restricted to certain accounts or entities. For instance, the P2ID and P2IDR note scripts target a specific account ID. Alternatively, Miden defines a `RECIPIENT` (represented as a `Word`) computed as:

```arduino
hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash)
```

Only those who know the `RECIPIENT`’s pre-image can consume the note. For private notes, this ensures an additional layer of control and privacy, as only parties with the correct data can claim the note.

The [transaction prologue](transactions/kernel.md) requires all necessary data to compute the note hash. This setup allows scenario-specific restrictions on who may consume a note.

For a practical example, refer to the [SWAP note script](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/note_scripts/SWAP.masm), where the `RECIPIENT` ensures that only a defined target can consume the swapped asset.

#### Note Nullifier - Ensuring Private Consumption

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

### Note Validation

Once created, a note must be validated by a Miden operator. Validation involves checking the transaction proof that produced the note to ensure it meets all protocol requirements.

- **Private Notes:** Only the note’s hash is recorded on-chain, keeping the data confidential.
- **Public Notes:** The full note data is stored, providing transparency for applications requiring public state visibility.

After validation, notes become “live” and eligible for discovery and eventual consumption.

### Note Discovery

Clients often need to find specific notes of interest. Miden allows clients to query the note database using note tags. These lightweight, 32-bit tags serve as best-effort filters, enabling quick lookups for notes related to particular use cases, scripts, or account prefixes.

The two most significant bits of the note tag guide its interpretation:

| Prefix | Execution hint | Target   | Allowed note type |
| ------ | :------------: | :------: | :----------------:|
| `0b00` | Network        | Specific | NoteType::Public  |
| `0b01` | Network        | Use case | NoteType::Public  |
| `0b10` | Local          | Any      | NoteType::Public  |
| `0b11` | Local          | Any      | Any               |

- **Execution hint:** Indicates whether the note is meant for network or local transactions.
- **Target:** Describes how to interpret the bits in the note tag. For tags with a specific target, the rest of the tag is interpreted as an account_id. For use case values, the meaning of the rest of the tag is not specified by the protocol and can be used by applications built on top of the rollup.
- **Allowed note type:** Specifies the note's storage mode, either `public` or `private`

> Example:
>
> The following 30 bits can represent anything. In the above example note tag, it represents an account Id of a public account. As designed the first bit of a public account is always `0` which overlaps with the second most significant bit of the note tag.
>
>```
>0b00000100_11111010_01010160_11100020
>```
>
>This example note tag indicates that the network operator (Miden node) executes the note against a specific account - `0x09f4adc47857e2f6`. Only the 30 most significant bits of the account id are represented in the note tag, since account Ids are 64-bit values but note tags only have 32-bits. Knowing a 30-bit prefix already narrows the set of potential target accounts down enough.

Using note tags strikes a balance between privacy and efficiency. Without tags, querying a specific note ID reveals a user’s interest to the operator. Conversely, downloading and filtering all registered notes locally is highly inefficient. Tags allow users to adjust their level of privacy by choosing how broadly or narrowly they define their search criteria, letting them find the right balance between revealing too much information and incurring excessive computational overhead.

### Note Consumption

To consume a note, the consumer must know its data, including the inputs needed to compute the nullifier. Consumption occurs as part of a transaction. Upon successful consumption a nullifier is generated for the consumed notes.

Upon successful verification of the transaction:

1. The Miden operator records the note’s nullifier as “consumed” in the nullifier database.
2. The note’s one-time claim is thus extinguished, preventing reuse.

## Conclusion

Miden’s notes introduce a powerful mechanism for secure, flexible, and private state management. By enabling asynchronous asset transfers, parallel execution, and privacy at scale, they transcend the limitations of strictly account-based models. As a result, developers and users alike enjoy enhanced scalability, confidentiality, and control. With these capabilities, Miden is paving the way for true **programmable money** where assets, logic, and trust converge seamlessly.
