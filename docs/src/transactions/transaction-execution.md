# Transaction Execution
Transaction are being executed by the Miden Transaction Executor. Transaction execution results in a **ExecutedTransaction** object and consists of the following steps:

1. Fetch the data required to execute a transaction from the **DataStore**.
2. Compile the transaction into a program using the **TransactionCompiler**.
3. Execute the transaction program and create an **ExecutedTransaction**.

The ExecutedTransaction is then being proven by the **TransactionProver**.

## The Data Store and Transaction Inputs
The **DataStore** defines the interface that transaction objects use to fetch data required for transaction execution. By calling `get_transaction_inputs()` it returns account, chain, and input note data required to execute a transaction against the account with the specified ID consuming the set of specified input notes. 

The **DataStore** must provide the following transaction inputs:

- the **Account** including the [AccountID](https://0xpolygonmiden.github.io/miden-base/architecture/accounts.html#account-id) and the [AccountCode](https://0xpolygonmiden.github.io/miden-base/architecture/accounts.html#code) which will be executed during the transaction execution. 
- the **BlockHeader**, which contains metadata about the block, commitments to the current state of the chain and the hash of the proof that attests to the integrity of the chain.
- the **ChainMmr**, which allows for efficient authentication of consumed notes during transaction execution. Authentication is achieved by providing an inclusion proof for the consumed notes in the transaction against the ChainMmr root associated with the latest block known at the time of transaction execution.  
- the **InputNotes** that are being consumed in the transaction (InputNotes), including the corresponding note data, e.g. the [note script](https://0xpolygonmiden.github.io/miden-base/architecture/notes.html#script) and [serial number](https://0xpolygonmiden.github.io/miden-base/architecture/notes.html#serial-number).

_Note: The InputNotes must all be recorded onchain for a successful transaction._

## The Transaction Compiler
Every transaction must be executed within the Miden VM to generate a transaction proof. In Miden there is a proof for every transaction. The transaction compiler is responsible for building executable programs. The generated programs - MASM programs - can then be executed on the Miden VM to update the states of accounts involved in these transactions. In addition to transaction compilation, the transaction compiler provides methods which can be used to compile Miden account code, note scripts, and transaction scripts. 

The `compile_transaction()` function results in an executable MASM Program, including the provided account interface and notes, an optional transaction script and the Transaction Kernel Program. The Transaction Kernel Program defines procedures and the memory layout for all parts of the transaction. A detailed description can be found below. 

Finally, after the transaction program has been compiled and the inputs including the advice prodiver were correctly populated, the transaction can be executed using the Miden VM processor.

## The Executed Transaction and the Transaction Outputs
The ExecutedTransaction object represents the result of a transaction - not its proof yet. From it, the account, and storage delta can be extracted. Furthermore, it serves as an input of the transaction prover to generate the proof. A successfully executed transaction results in a new state of the provided account, a vector of all created Notes and a vector of all the consumed Notes and their Nullifiers.

The transaction outputs consists of 

- the **AccountStub** which is a stub of an account which contains information that succinctly describes the state of the components of the account.
- the **OutputNotes** vector that contains a list of output notes of a transaction. The vector can be empty if the transaction does not produce any notes.

## The Transaction Prover
The Transaction Prover proves the provided transaction and returns a ProvenTransaction object. This object can be verified by the Miden Node using the Transaction Verifier structure and if valid updating the [State](../architecture/state.md) databases.
