# Note lifecycle
For a note to exist it must be present in the [Notes DB](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#notes-database) kept by the Miden Node(s). New notes are being produced when executing a transaction. They can be produced locally by users in local transactions or by the operator in a network transaction.

The lifcycle of a note is as follows:
* A new note is produced when a transaction is executed - regardless of the transaction type
* Operator will receive the note hash and if the note is public, it'll also receive the corresponding note's data
* Operator verifies the correctness of the underlying transaction before adding the note hash to the Notes DB
* The note can now be consumed in a seperate transaction - to consume the note, the note's data must be known
* A note is consumed when the its nullifier in the [Nullifier DB](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#nullifier-database) is set to `1`
* Operator will receive the note's nullifier together with a transaction proof
* After successful verification, the Operator sets the corresponding entry in the Nullifier DB to `1`