# Transaction Kernel Program

The transaction kernel program, written in [MASM](https://0xpolygonmiden.github.io/miden-vm/user_docs/assembly/main.html), is responsible for executing a Miden rollup transaction within the Miden VM. It is defined as a MASM [kernel](https://0xpolygonmiden.github.io/miden-vm/user_docs/assembly/execution_contexts.html#kernels).

The kernel provides context-sensitive security, preventing unwanted read and write access. It defines a set of procedures which can be invoked from other [contexts](https://0xpolygonmiden.github.io/miden-vm/user_docs/assembly/execution_contexts.html#execution-contexts); e.g., notes executed in the root context.

In general, the kernel's procedures must reflect everything users might want to do while executing transactions—from transferring assets to complex smart contract interactions with custom code.

> **Info**
> - Learn more about Miden transaction [procedures](procedures.md) and [contexts](contexts.md).

The kernel has a well-defined structure which does the following:

1. The [prologue](#prologue) prepares the transaction for processing by parsing the transaction data and setting up the root context.
2. Note processing executes the note processing loop which consumes each `InputNote` and invokes the note script of each note.
3. Transaction script processing executes the optional transaction script.
4. The [epilogue](#epilogue) finalizes the transaction by computing the output notes commitment, the final account hash, asserting asset invariant conditions, and asserting the nonce rules are upheld.

![Transaction program](../../img/architecture/transaction/transaction-program.png)

## Input

The transaction kernel program receives two types of inputs: public inputs via the `operand_stack` and private inputs via the `advice_provider`.

- **Operand stack**: Holds the global inputs which serve as a commitment to the data being provided via the advice provider.
- **Advice provider**: Holds data of the last known block, account, and input note data.

## Prologue

The transaction prologue executes at the beginning of a transaction. It performs the following tasks:

1. _Unhashes_ the inputs and lays them out in the root context memory.
2. Builds a single vault (transaction vault) containing assets of all inputs (input notes and initial account state).
3. Verifies that all input notes are present in the note DB.

The memory layout is illustrated below. The kernel context has access to all memory slots.

![Memory layout kernel](../../img/architecture/transaction/memory-layout-kernel.png)

### Bookkeeping section

Tracks variables used internally by the transaction kernel.

### Global inputs

Stored in pre-defined memory slots. Global inputs include the block hash, account ID, initial account hash, and nullifier commitment.

### Block data

Block data, read from the advice provider, is stored in memory. The block hash is computed and verified against the global inputs.

### Chain data

Chain root is recomputed and verified against the chain root in the block data section.

### Account data

Reads data from the advice provider, stores it in memory, and computes the account hash. The hash is validated against global inputs. For new accounts, initial account hash and validation steps are applied.

### Input note data

Processes input notes by reading data from advice providers and storing it in memory. It computes the note's hash and nullifier, forming a transaction nullifier commitment.

> **Info**
> - Note data is required for computing the nullifier (e.g., the [note script](../notes.md#main-script) and serial number).
> - Note recipients define the set of users who can consume specific notes.

## Note Processing

Notes are consumed in a loop, invoking their scripts in isolated contexts using `dyncall`.

```arduino
# loop while we have notes to consume
while.true
    exec.note::prepare_note
    dyncall
    dropw dropw dropw dropw
    exec.note::increment_current_input_note_ptr
    loc_load.0
    neq
end
```

When processing a note, new note creation may be triggered, and information about the new note is stored in the output note data.

> **Info**
> - Notes can only call account interfaces to trigger write operations, preventing direct access to account storage.

## Transaction Script Processing

If provided, the transaction script is executed after all notes are consumed. The script may authenticate the transaction by increasing the account nonce and signing the transaction.

```arduino
use.miden::contracts::auth::basic->auth_tx

begin
    padw padw padw padw
    call.auth_tx::auth_tx_rpo_falcon512
    dropw dropw dropw dropw
end
```

> **Note**
> - The account must expose the `auth_tx_rpo_falcon512` function for the transaction script to call it.

## Epilogue

Finalizes the transaction:

1. Computes the final account hash.
2. Asserts that the final account nonce is greater than the initial nonce if the account has changed.
3. Computes the output notes commitment.
4. Asserts that input and output vault roots are equal (except for special accounts like faucets).

## Outputs

The transaction kernel program outputs:

1. The transaction script root.
2. A commitment of all newly created output notes.
3. The account hash in its new state.
