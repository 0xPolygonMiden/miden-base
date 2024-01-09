# The Transaction Kernel Program
The transaction kernel program has a well-defined structure which must do the following:

1. **Prologue**: execute the transaction prologue which prepares the transaction for processing by parsing the transaction data and setting up the root context.
2. **Note Processing**: execute the note processing loop which consumes each input note and invokes the note script of each note.
3. **Transaction Script Processing**: execute the transaction script if it exists.
4. **Epilogue**: execute the transaction epilogue which finalizes the transaction by computing the created notes commitment, the final account hash, asserting asset invariant conditions and asserting the nonce rules are upheld.

<p align="center">
    <img src="../diagrams/architecture/transaction/Transaction_program.png" style="width: 75%;">
</p>

## The Prologue
The Prologue stores all provided information from the advice provider into the appropriate memory slots. It then asserts if the provided block, chain and note data equals the provided commitments. 

First, all **global inputs** are being stored in the pre-defined memory slots. Global inputs are being provided via the `operand_stack` to the VM at transaction execution. They serve as a commitment to the data being provided via the advice provider. Global inputs are the Block Hash, the Account ID, the initial Account Hash, and the Nullifier Commitment. This is a sequential hash of all (nullifier, script_root) pairs for the notes consumed in the transaction.

Second, the **block data** is being processed. This involves reading the data from the advice provider and storing it at the appropriate memory addresses. As the block data is read from the advice provider, the block hash is computed. It is asserted that the computed block hash matches the block hash stored in the global inputs.

Third, the **chain data** is being processed in a similar way as the block data. In this case the chain root is being recomputed and compared against the chain root stored in the block data section. 

Fourth, the **account data** is being processed. This involves reading the data from the advice provider and storing it at the appropriate memory addresses. As the account data is read from the advice provider, the account hash is computed.  If the account is new then the global initial account hash is updated and the new account is validated.  If the account already exists then it is asserted that the computed account hash matches the account hash provided via global inputs. It is also asserted that the account id matches the account id provided via the stack public inputs.

Fifth, the **input notes** are being processed. This involves per note reading the data from the advice provider and storing it at the appropriate memory addresses. As each note is consumed its hash and nullifier is computed. The transaction nullifier commitment is computed via a sequential hash of all (nullifier, ZERO) pairs for all consumed notes. This step involves authentication that the input note data provided via the advice provider is consistent with the chain history. 

_Note: One needs to provide the note data to compute the nullifier, e.g. the [note script](https://0xpolygonmiden.github.io/miden-base/architecture/notes.html#script) and [serial number](https://0xpolygonmiden.github.io/miden-base/architecture/notes.html#serial-number). So you need to know the note data to execute the prologue of a transaction. This is how the [note recipient](https://0xpolygonmiden.github.io/miden-base/architecture/notes.html#note-recipient) defines the set of users who can consume a specific note. The executing account needs to provide the pre-image data to the recipient at the time of execution._

Lastly, if a transaction script is provided, its root is being stored at the pre-defined memory address. 

## The Note Processing
If there are input notes they are being consumed in a loop. For every note, the [MAST root](https://0xpolygonmiden.github.io/miden-vm/design/programs.html) of the note script is being loaded onto the stack. Then, by calling a [`dyncall`](https://0xpolygonmiden.github.io/miden-vm/user_docs/assembly/code_organization.html?highlight=dyncall#dynamic-procedure-invocation) the note script is being executed with respect to the necessary context switches to prevent unwanted memory access. 

```
    # loop while we have notes to consume
    while.true
        # execute the note setup script
        exec.note::prepare_note
        # => [NOTE_SCRIPT_HASH]

        # invoke the note script using the dyncall instruction
        dyncall
        # => [OUTPUT_3, OUTPUT_2, OUTPUT_1, OUTPUT_0]

        # clean up note script outputs
        dropw dropw dropw dropw
        # => []

        # check if we have more notes to consume and should loop again
        exec.note::increment_current_consumed_note_ptr
        loc_load.0
        neq
        # => [should_loop]
    end
```

_Note: The Miden Transaction Kernel Program prevents notes from having direct access to account storage. Notes can only call the account interface to trigger write / read operations in the account._

## The Transaction Script Processing
If there is a transcation script provided with the transaction, it will be processed after all notes are being consumed. By loading the transaction script root onto the stack the kernel can invoke a dyncall and in doing so execute the script. The transaction script can be used to authenticate the transaction by increasing the account's nonce and signing the transaction, see the following example:

```
    use.miden::contracts::auth::basic->auth_tx

    begin
        call.auth_tx::auth_tx_rpo_falcon512
    end
```
_Note: The executing account must expose the `auth_tx` function in order for the transaction script to call it._


## The Epilogue
The Epilogue checks if the transaction outputs are valid. It asserts that the input and output vault roots are equal, so no assets are being destroyed or minted (there is an exeption for special accounts, called faucets). It also checks that the account's nonce has changed, and it computed the created notes commitment in case new notes were created. 