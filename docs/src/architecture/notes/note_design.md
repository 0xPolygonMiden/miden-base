# Note design
The diagram below illustrates the contents of a note:

<p align="center">
    <img src="../../diagrams/architecture/note/Note.png">
</p>

As shown in the above picture:
* **Vault &rarr;** a set of assets that are stored in note
* **Script &rarr;** it must be executed in a context of some account to claim the assets
* **Inputs &rarr;** these are placed onto the stack before a note's script is executed
* **Serial number &rarr;** a note's unique identifier

## Vault
Asset container for a note. A note vault can contain up to `255` assets stored in an array. The entire vault can be reduced to a single hash which is computed by sequentially hashing the list of the vault's [assets](../assets.md).

## Script
Single executable script. This script will be executed in a [transaction](https://0xpolygonmiden.github.io/miden-base/architecture/transactions.html). This script is also the root of a [Miden program MAST](https://0xpolygonmiden.github.io/miden-vm/user_docs/assembly/main.html). A script is always executed in the context of a single account, and thus, may invoke zero or more the [account's functions](https://0xpolygonmiden.github.io/miden-base/architecture/accounts.html#code).

*Note: Since code in Miden is expresed as MAST, every function is a commitment to the underlying code. The code cannot change unnoticed to the user because its hash would change.*

## Inputs
A note script can take parameters (passed via the stack) as inputs.

## Serial number
A note's serial number identifies the note and this is needed to create the note's hash and nullifier. The serial number is represented by a `Word` and can be chosen by the user at note creation. The serial number is used to break linkability between note hash and note nullifier. 