# Notes
A note is a way of transferring assets between accounts. A note consists of a vault and a script as shown in the diagram below.

<p align="center">
    <img src="../diagrams/architecture/note/Note.png">
</p>

In the above:
* **Vault** is a set of assets that are stored in note.
* **Script** that must be executed in a context of some account to claim the assets.
* **Inputs** which are placed onto the stack before a note's script is executed.
* **Serial number** is the note's identifier.

## Vault
Asset container for a note. A note vault can contain up to 255 assets. The entire vault can be reduced to a single hash which is computed by sequentially hashing the list of the vault's assets.

A note's vault is basically the same as an account's vault.

## Script
Unlike an account, a note has a single executable script. This script is also a root of a [Miden program MAST](https://0xpolygonmiden.github.io/miden-vm/user_docs/assembly/main.html). A script is always executed in the context of a single account, and thus, may invoke account's functions. A note script does not have to call any of account's functions. More generally, a note's script can call zero or more of an account's function.

## Inputs
A note script can take parameters (passed via the stack) as inputs.

## Serial number
A note's serial number identifies the note and this is needed to create the note's hash and nullifier. The serial number is used to break linkability between note hash and note nullifier.

# Note metadata
For every note the Miden Node stores metadata in the Note DB. This metadata includes:
* user-defined tag (1 field element)
* sender (1 field element)
* number of assets contained in the note (1 field element)
____

# Types of notes
There are two types of notes in Miden. Notes can be stored privately in the [Notes DB](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#notes-database) with only the note hash. Or notes can be stores publicly with all data.

# Note hash
The note hash is computed as:

`hash(hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash), vault_hash)`

This achieves the following properties:
- Every note can be reduced to a single unique hash.
- To compute a note's hash, we do not need to know the note's serial_num. Knowing the hash
    of the serial_num (as well as script hash, input hash and note vault) is sufficient.
- Moreover, we define `recipient` as: \
    `hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash)`. This allows computing the note hash from recipient and note vault.
- We compute the hash of `serial_num` as `hash(serial_num, [0; 4])` to simplify processing within
the VM.

# Note nullifier
The nullifier is the note's index in the [Nullifier DB](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#nullifier-database). The Nullifier DB stores the information whether a note was already consumed.

The nullifier is computed as `hash(serial_num, script_hash, input_hash, vault_hash)`.
This achieves the following properties:
- Every note can be reduced to a single unique nullifier.
- We cannot derive a note's hash from its nullifier.
- To compute the nullifier we must know all components of the note: `serial_num`, `script_hash`, `input_hash` and `vault_hash`.

# Note lifecycle
For a note to exist it must be present in the [Notes DB](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#notes-database) kept by the Miden Node(s). New notes are being produced when executing a transaction. They can be produced locally by users in local transactions or by the operator in a network transaction.

The lifcycle is as follows:
* A new note is being produced when a transaction is executed - regardless of the transaction type
* Eventually the Operator will receive the note hash and if the note is public also the corresponding note's data
* The Operator verifies the correctness of the underlying transaction before adding the note hash to the Notes DB
* The note can now be consumed in a seperate transaction - to consume the note, the note's data must be known
* A note is consumed when the its nullifier in the [Nullifier DB](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#nullifier-database) is set to `1`
* Eventually the Operator will receive the note's nullifier together with a transaction proof
* After successful verification the Operator sets the corresponding entry in the Nullifier DB to `1`
