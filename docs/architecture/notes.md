# Note

> The medium through which [Accounts](accounts.md) communicate in the Miden protocol.

## What is the purpose of a note?

In Miden's hybrid UTXO and account-based model `Note`s represent UTXO's which enable parallel transaction execution and privacy through asynchronous local `Note` production and consumption. 

## What is a note?

A `Note` in Miden holds assets and defines how these assets can be consumed.

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

> An [asset](assets.md) container for a `Note`.

A `Note` can contain up to 256 different assets. These assets represent fungible or non-fungible tokens, enabling flexible asset transfers.

### Script

> The code executed when the `Note` is consumed.

Each `Note` has a script that defines the conditions under which it can be consumed. When accounts consume `Note`s in transactions, `Note` scripts call the account’s interface functions. This enables all sorts of operations beyond simple asset transfers. The Miden VM’s Turing completeness allows for arbitrary logic, making `Note` scripts highly versatile.

### Inputs

> Arguments passed to the `Note` script during execution.

A `Note` can have up to 128 input values, which adds up to a maximum of 1 KB of data. The `Note` script can access these inputs. They can convey arbitrary parameters for `Note` consumption. 

### Serial number

> A unique and immutable identifier for the `Note`.

The serial number has two main purposes. Firstly by adding some randomness to the `Note` it ensures it's uniqueness, secondly in private `Note`s it helps prevent linkability between the `Note`'s hash and its nullifier. The serial number should be a random 32 bytes number chosen by the user. If leaked, the `Note`’s nullifier can be easily computed, potentially compromising privacy.

### Metadata

> Additional `Note` information.

`Note`s include metadata such as the sender’s account ID and a [tag](#note-discovery) that aids in discovery. Regardless of [storage mode](#note-storage-mode), these metadata fields remain public.

## Note Lifecycle

![Architecture core concepts](../img/architecture/note/note-life-cycle.png)

The `Note` lifecycle proceeds through four primary phases: **creation**, **validation**, **discovery**, and **consumption**. Throughout this process, `Note`s function as secure, privacy-preserving vehicles for asset transfers and logic execution.

### Note creation

Accounts can create `Note`s in a transaction. The `Note` exists if it is included in the global `Note`s DB.

- **Users:** Executing local or network transactions.
- **Miden operators:** Facilitating on-chain actions, e.g. such as executing user `Note`s against a DEX or other contracts.

#### Note storage mode

As with [accounts](accounts.md), `Note`s can be stored either publicly or privately:

- **Public mode:** The `Note` data is stored in the [note database](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#notes-database), making it fully visible on-chain.
- **Private mode:** Only the `Note`’s hash is stored publicly. The `Note`’s actual data remains off-chain, enhancing privacy.

#### Ephemeral note

These specific `Note`s can be consumed even if not yet registered on-chain. They can be chained together into one final proof. This can allow for example sub-second communication below blocktimes by adding additional trust assumptions.

### Note validation

Once created, a `Note` must be validated by a Miden operator. Validation involves checking the transaction proof that produced the `Note` to ensure it meets all protocol requirements.

- **Private Notes:** Only the `Note`’s hash is recorded on-chain, keeping the data confidential.
- **Public Notes:** The full `Note` data is stored, providing transparency for applications requiring public state visibility.

After validation, `Note`s become “live” and eligible for discovery and eventual consumption.

### Note discovery

Clients often need to find specific `Note`s of interest. Miden allows clients to query the `Note` database using `Note` tags. These lightweight, 32-bit data fields serve as best-effort filters, enabling quick lookups for `Note`s related to particular use cases, scripts, or account prefixes.

    As discussed in detail in [transaction kernel procedures](transactions/procedures.md) certain procedures can only be invoked in certain contexts. The note script is being executed in the note context of the [transaction kernel](transactions/kernel.md).

    #### Main script

    The main part of the P2ID script checks if the executing account is the same as the account defined in the `NoteInputs`. The creator of the note defines the note script and the note inputs separately to ensure usage of the same standardized P2ID script regardless of the target account ID. That way, it is enough to check the script root (see above).

    ```arduino
    # Pay-to-ID script: adds all assets from the note to the account, assuming ID of the account
    # matches target account ID specified by the note inputs.
    #
    # Requires that the account exposes: miden::contracts::wallets::basic::receive_asset procedure.
    #
    # Inputs: [SCRIPT_ROOT]
    # Outputs: []
    #
    # Note inputs are assumed to be as follows:
    # - target_account_id is the ID of the account for which the note is intended.
    #
    # FAILS if:
    # - Account does not expose miden::contracts::wallets::basic::receive_asset procedure.
    # - Account ID of executing account is not equal to the Account ID specified via note inputs.
    # - The same non-fungible asset already exists in the account.
    # - Adding a fungible asset would result in amount overflow, i.e., the total amount would be
    #   greater than 2^63.
    begin
        # drop the transaction script root
        dropw
        # => []

        # load the note inputs to memory starting at address 0
        push.0 exec.note::get_inputs
            # => [inputs_ptr]

        # read the target account id from the note inputs
        mem_load
        # => [target_account_id]

        exec.account::get_id
        # => [account_id, target_account_id, ...]

        # ensure account_id = target_account_id, fails otherwise
        assert_eq
        # => [...]

        exec.add_note_assets_to_account
        # => [...]
    end
    ```

    1. Every note script starts with the note script root on top of the stack. 
    2. After the `dropw`, the stack is cleared. 
    3. Next, the script stored the note inputs at pos 0 in the [relative note context memory](https://0xpolygonmiden.github.io/miden-base/transactions/transaction-procedures.html#transaction-contexts) by `push.0 exec.note::get_inputs`. 
    4. Then, `mem_load` loads a `Felt` from the specified memory address and puts it on top of the stack, in that cases the `target_account_id` defined by the creator of the note.
    5. Now, the note invokes `get_id` from the account API using `exec.account::get_id` - which is possible even in the note context. 

    Because, there are two account IDs on top of the stack now, `assert_eq` fails if the two account IDs (target_account_id and executing_account_id) are not the same. That means, the script cannot be successfully executed if executed by any other account than the account specified by the note creator using the note inputs.

    If execution hasn't failed, the script invokes a helper procedure `exec.add_note_assets_to_account` to add the note's assets into the executing account's vault.

    #### Add assets

    This procedure adds the assets held by the note into the account's vault.

    ```arduino
    #! Helper procedure to add all assets of a note to an account.
    #!
    #! Inputs: []
    #! Outputs: []
    #!
    proc.add_note_assets_to_account
        push.0 exec.note::get_assets
        # => [num_of_assets, 0 = ptr, ...]

        # compute the pointer at which we should stop iterating
        dup.1 add
        # => [end_ptr, ptr, ...]

        # pad the stack and move the pointer to the top
        padw movup.5
        # => [ptr, 0, 0, 0, 0, end_ptr, ...]

        # compute the loop latch
        dup dup.6 neq
        # => [latch, ptr, 0, 0, 0, 0, end_ptr, ...]

        while.true
            # => [ptr, 0, 0, 0, 0, end_ptr, ...]

            # save the pointer so that we can use it later
            dup movdn.5
            # => [ptr, 0, 0, 0, 0, ptr, end_ptr, ...]

            # load the asset
            mem_loadw
            # => [ASSET, ptr, end_ptr, ...]
            
            # pad the stack before call
            padw swapw padw padw swapdw
            # => [ASSET, pad(12), ptr, end_ptr, ...]

            # add asset to the account
            call.wallet::receive_asset
            # => [pad(16), ptr, end_ptr, ...]

            # clean the stack after call
            dropw dropw dropw
            # => [0, 0, 0, 0, ptr, end_ptr, ...]

            # increment the pointer and compare it to the end_ptr
            movup.4 add.1 dup dup.6 neq
            # => [latch, ptr+1, 0, 0, 0, 0, end_ptr, ...]
        end

        # clear the stack
        drop dropw drop
    end
    ```

    The procedure starts by calling `exec.note::get_assets`. As with the note's inputs before, this writes the assets of the note into memory starting at the specified address. Assets are stored in consecutive memory slots, so `dup.1 add` provides the last memory slot.

    In Miden, [assets](assets.md) are represented by `Words`, so we need to pad the stack with four `0`s to make room for an asset. Now, if there is at least one asset (checked by `dup dup.6 neq`), the loop starts. It first saves the pointer for later use (`dup movdn.5`), then loads the first asset `mem_loadw` on top of the stack.

    Now, the procedure calls the a function of the account interface `call.wallet::receive_asset` to put the asset into the account's vault. Due to different [contexts](https://0xpolygonmiden.github.io/miden-base/transactions/transaction-procedures.html#transaction-contexts), a note script cannot directly call an account function to add the asset. The account must expose this function in its [interface](https://0xpolygonmiden.github.io/miden-base/architecture/accounts.html#example-account-code).

    Lastly, the pointer gets incremented, and if there is a second asset, the loop continues (`movup.4 add.1 dup dup.6 neq`). Finally, when all assets were put into the account's vault, the stack is cleared (`drop dropw drop`).

### Note storage mode

Similar to accounts, there are two storage modes for notes in Miden - private and public. Notes can be stored publicly in the [note database](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#notes-database) with all data publicly visible for everyone. Alternatively, notes can be stored privately by committing only the note hash to the note database.

Every note has a unique note hash. It is defined as follows:

```arduino
hash(hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash), vault_hash)
```

!!! info
    To compute a note's hash, we do not need to know the note's `serial_num`. Knowing the hash of the `serial_num` (as well as `script_hash`, `input_hash` and `note_vault`) is also sufficient. We compute the hash of `serial_num` as `hash(serial_num, [0; 4])` to simplify processing within the VM._

### Note discovery (note tags)

Note discovery describes the process by which Miden clients find notes they want to consume. Miden clients can query the Miden node for notes carrying a certain note tag in their metadata. Note tags are best-effort filters for notes registered on the network. They are lightweight values (32-bit) used to speed up queries. Clients can follow tags for specific use cases, such as swap scripts, or user-created custom tags. Tags are also used by the operator to identify notes intended for network execution and include the corresponding information on how to execute them.

The two most signification bits of the note tag have the following interpretation:

| Prefix | Execution hint | Target   | Allowed note type |
| ------ | :------------: | :------: | :----------------:|
| `0b00` | Network        | Specific | NoteType::Public  |
| `0b01` | Network        | Use case | NoteType::Public  |
| `0b10` | Local          | Any      | NoteType::Public  |
| `0b11` | Local          | Any      | Any               |

- Execution hint: Set to `Network` for network transactions. These notes are validated and, if possible, consumed in a network transaction.
- Target: Describes how to interpret the bits in the note tag. For tags with a specific target, the rest of the tag is interpreted as an `account_id`. For use case values, the meaning of the rest of the tag is not specified by the protocol and can be used by applications built on top of the rollup.
- Allowed note type: Describes the note's storage mode, either `public` or `private`.

The following 30 bits can represent anything. In the above example note tag, it represents an account Id of a public account. As designed the first bit of a public account is always `0` which overlaps with the second most significant bit of the note tag.

```
0b00000100_11111010_01010110_11100010
```

This example note tag indicates that the network operator (Miden node) executes the note against a specific account - `0x09f4adc47857e2f6`. Only the 30 most significant bits of the account id are represented in the note tag, since account Ids are 64-bit values but note tags only have 32-bits. Knowing a 30-bit prefix already narrows the set of potential target accounts down enough.

Using note tags is a compromise between privacy and latency. If a user queries the operator using the note ID, the operator learns which note a specific user is interested in. Alternatively, if a user always downloads all registered notes and filters locally, it is quite inefficient. By using tags, users can customize privacy parameters by narrowing or broadening their note tag schemes.

??? note "Example note tag for P2ID"
    P2ID scripts can only be consumed by the specified account ID (target ID). In the standard schema, the target ID is encoded into the note tag.

    For network execution of a P2ID note, the note tag is encoded as follows: 0b00000100_11111010_01010110_11100010. This encoding allows the Miden operator to quickly identify the account against which the transaction must be executed.

    For local execution of a P2ID note, the recipient needs to be able to discover the note. The recipient can query the Miden node for a specific tag to see if there are new P2ID notes to be consumed. In this case, the two most significant bits are set to 0b11, allowing any note type (private or public) to be used. The next 14 bits represent the 14 most significant bits of the account ID, and the remaining 16 bits are set to 0.

    Example for local execution:
    ```
    0b11000100_11111010_00000000_00000000
    ```
    This "fuzzy matching" approach balances privacy and efficiency. A note with this tag could be intended for any account sharing the same 16-bit prefix.

### Note consumption

To consume a `Note`, the consumer must know its data, including the inputs needed to compute the nullifier. Consumption occurs as part of a transaction. Upon successful consumption a nullifier is generated for the consumed `Note`s.

Upon successful verification of the transaction:

1. The Miden operator records the `Note`’s nullifier as “consumed” in the nullifier database.
2. The `Note`’s one-time claim is thus extinguished, preventing reuse.

#### Note recipient restricting consumption

Consumption of a `Note` can be restricted to certain accounts or entities. For instance, the P2ID and P2IDR `Note` scripts target a specific account ID. Alternatively, Miden defines a RECIPIENT (represented as 32 bytes) computed as:

```arduino
hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash)
```

Only those who know the RECIPIENT’s pre-image can consume the `Note`. For private `Note`s, this ensures an additional layer of control and privacy, as only parties with the correct data can claim the `Note`.

The [transaction prologue](transactions/kernel.md) requires all necessary data to compute the `Note` hash. This setup allows scenario-specific restrictions on who may consume a `Note`.

For a practical example, refer to the [SWAP note script](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/note_scripts/SWAP.masm), where the RECIPIENT ensures that only a defined target can consume the swapped asset.

#### Note nullifier ensuring private consumption

The `Note` nullifier, computed as:

```arduino
hash(serial_num, script_hash, input_hash, vault_hash)
```

This achieves the following properties:

- Every `Note` can be reduced to a single unique nullifier.
- One cannot derive a `Note`'s hash from its nullifier.
- To compute the nullifier, one must know all components of the `Note`: serial_num, script_hash, input_hash, and vault_hash.

That means if a `Note` is private and the operator stores only the `Note`'s hash, only those with the `Note` details know if this `Note` has been consumed already. Zcash first [introduced](https://zcash.github.io/orchard/design/nullifiers.html#nullifiers) this approach.

![Architecture core concepts](../img/architecture/note/nullifier.png)

## Conclusion

Miden’s `Note` introduce a powerful mechanism for secure, flexible, and private state management. By enabling asynchronous asset transfers, parallel execution, and privacy at scale, `Note`s transcend the limitations of strictly account-based models. As a result, developers and users alike enjoy enhanced scalability, confidentiality, and control. With these capabilities, Miden is paving the way for true **programmable money** where assets, logic, and trust converge seamlessly.
