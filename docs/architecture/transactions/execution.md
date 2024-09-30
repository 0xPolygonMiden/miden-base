---
comments: true
---

The Miden transaction executor is the component that executes transactions. 

Transaction execution consists of the following steps and results in an `ExecutedTransaction` object:

1. Fetch the data required to execute a transaction from the data store.
2. Compile the transaction into an executable [MASM](https://0xpolygonmiden.github.io/miden-vm/user_docs/assembly/main.html) program using the transaction compiler.
3. Execute the transaction program and create an `ExecutedTransaction` object.
4. Prove the `ExecutedTransaction` using the transaction prover.

<center>
![Transaction execution process](../../img/architecture/transaction/transaction-execution-process.png)
</center>

One of the main reasons for separating out the execution and proving steps is to allow _stateless provers_; i.e. the executed transaction has all the data it needs to re-execute and prove a transaction without database access. This supports easier proof-generation distribution.

## Data store and transaction inputs

The data store defines the interface that transaction objects use to fetch the data for transaction executions. Specifically, it provides the following inputs to the transaction:

- `Account` data which includes the [AccountID](../accounts.md#account-id) and the [AccountCode](../accounts.md#code) that is executed during the transaction.
- A `BlockHeader` which contains metadata about the block, commitments to the current state of the chain, and the hash of the proof that attests to the integrity of the chain.
- A `ChainMmr` which authenticates input notes during transaction execution. Authentication is achieved by providing an inclusion proof for the transaction's input notes against the `ChainMmr`-root associated with the latest block known at the time of transaction execution.
- `InputNotes` consumed by the transaction that include the corresponding note data, e.g. the [note script](../notes.md#the-note-script) and serial number.

!!! note
    - `InputNotes` must be already recorded on-chain in order for the transaction to succeed.
    - There is no nullifier-check during a transaction. Nullifiers are checked by the Miden operator during transaction verification. So at the transaction level, there is "double spending".

## Transaction compiler

Every transaction is executed within the Miden VM to generate a transaction proof. In Miden, there is a proof for every transaction. 

The transaction compiler is responsible for building executable programs. The generated MASM programs can then be executed by the Miden VM which generates a zk-proof. In addition to transaction compilation, the transaction compiler provides methods for compiling Miden account code, note scripts, and transaction scripts.

Compilation results in an executable MASM program. The program includes the provided account interface and notes, an optional transaction script, and the [transaction kernel program](kernel.md). The transaction kernel program defines procedures and the memory layout for all parts of the transaction. 

After compilation, assuming correctly-populated inputs, including the advice provider, the transaction can be executed.

## Executed transactions and the transaction outputs

The `ExecutedTransaction` object represents the result of a transaction not its proof. From this object, the account and storage delta can be extracted. Furthermore, the `ExecutedTransaction` is an input to the transaction prover. 

A successfully executed transaction results in a new account state which is a vector of all created notes (`OutputNotes`) and a vector of all the consumed notes (`InputNotes`) together with their nullifiers.

## Transaction prover

The transaction prover proves the inputted `ExecutedTransaction` and returns a `ProvenTransaction` object. The Miden node verifies the `ProvenTransaction` object using the transaction verifier and, if valid, updates the [state](../state.md) databases.

<br/>
