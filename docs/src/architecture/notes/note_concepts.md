# Note storage modes
Similar to accounts, there are two storage modes for notes in Miden. Notes can be stored privately in the [Notes DB](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#notes-database) with only the note hash. Or notes can be stored publicly with all data.

Privately stored notes can only be consumed if the note data is known to the consumer. That means, there must be some offchain communication to transmit the note's data from the sender to the receipient.

# Note metadata
For every note the Miden Operator stores metadata in the Note DB. This metadata includes:

* A **user-defined tag** as a means to quickly grab all notes for a certain application or use case.
* A **sender** to be able to provide also ERC20 contract functionality.
* The **number of assets** contained in the note. 

# Note hash
The note hash is computed as:

`hash(hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash), vault_hash)`

This achieves the following properties:
- Every note can be reduced to a single unique hash.
- To compute a note's hash, we do not need to know the note's `serial_num`. Knowing the hash
    of the `serial_num` (as well as `script_hash`, `input_hash` and `note_vault`) is sufficient.
- Moreover, we define `recipient` as: `hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash)`. This allows computing the note hash from recipient and note vault.
- We compute the hash of `serial_num` as `hash(serial_num, [0; 4])` to simplify processing within
the VM.

# Note nullifier
The nullifier is the note's index in the [Nullifier DB](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#nullifier-database). The Nullifier DB stores the information whether a note was already consumed.

The nullifier is computed as `hash(serial_num, script_hash, input_hash, vault_hash)`.

This achieves the following properties:
- Every note can be reduced to a single unique nullifier.
- We cannot derive a note's hash from its nullifier.
- To compute the nullifier, we must know all components of the note: `serial_num`, `script_hash`, `input_hash`, and `vault_hash`.

To know a note’s nullifier, one needs to know all details of the note. That means if a note is private and the operator stores only the note's hash, only those with the note details know if this note has been consumed already. Zcash first introduced this approach.

<p align="center">
    <img src="../../diagrams/architecture/note/Nullifier.png">
</p>