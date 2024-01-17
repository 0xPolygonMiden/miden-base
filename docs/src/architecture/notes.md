# Notes
Miden aims to achieve parallel transaction execution and privacy. The UTXO-model combined with client-side proofs provide those features. That means, in Miden exist notes as a way of transferring assets between and to interact with accounts. Notes can be consumed and produced asynchronously and privately. The concept of notes is a key difference between Ethereum’s Account-based model and Polygon Miden, which uses a hybrid UTXO- and Account-based [state-model](state.md). 

# Note design
In Polygon Miden, accounts communicate with one another by producing and consuming notes. Notes are messages carrying assets that accounts can send to each other—a note stores assets and a script that defines how this note can be consumed. 

The diagram below illustrates the contents of a note:

<p align="center">
    <img src="../diagrams/architecture/note/Note.png" style="width: 50%;">
</p>

As shown in the above picture:
* **Assets &rarr;** serves as [asset](assets.md) container for a note. It can contain up to `255` assets stored in an array which can be reduced to a single hash.
* **Script &rarr;** will be executed in a [transaction](https://0xpolygonmiden.github.io/miden-base/architecture/transactions.html) against a single account.
* **Inputs &rarr;** are placed onto the stack as parameters before a note's script gets executed. They must be defined at note creation. 
* **Serial number &rarr;** a note's unique identifier to break linkability between [note hash](https://0xpolygonmiden.github.io/miden-base/architecture/notes.html#note-hash) and [nullifier](https://0xpolygonmiden.github.io/miden-base/architecture/notes.html#note-nullifier). Should be a random `Word` chosen by the user - if revealed, the nullifier might be computed easily.

 In addition, a note has **metadata** including the sender, tag, and number of assets.

# Note's lifecycle
New notes are being created when executing a transaction. After verifying the transaction proof the Operator adds either only the note hash or the full note data to the [Notes DB](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#notes-database). Notes can be produced and consumed locally by users in local transactions or by the operator in a network transaction. Note consumption requires the transacting party to know the note data to compute the nullifier. After successful verification, the Operator sets the corresponding entry in the Nullifier DB to `1`. 

<p align="center">
    <img src="../diagrams/architecture/note/Note_life_cycle.png">
</p>

The following section will explain, how notes are created, stored, discovered and consumed. 

## Note creation
Notes are created in Miden transactions. If a valid Miden transaction outputting an `OutputNote` gets verified by the Miden Operator it gets added to the Notes DB.

### Creating the note object 
Most users will create new note objects using the Miden Client (ToDo define how to). It is also possible to create note objects directly in [Rust](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/src/notes/mod.rs) or by using a [transaction kernel procedure](https://github.com/0xPolygonMiden/miden-base/blob/e4cba6a5d9581f76604cd89bd05d129b3c84e254/miden-lib/asm/kernels/transaction/api.masm#L421) in MASM. 


## The note script 
Every note has a script which gets executed at note consumption. The script allows for more than just the transferring of assets. It is always executed in the context of a single account, and thus, may invoke zero or more of the [account's functions](https://0xpolygonmiden.github.io/miden-base/architecture/accounts.html#code). Note scripts can become arbirarly complex due to the underlying Turing-complete Miden VM. 

In `miden-lib` there are [predefined note scripts](https://github.com/0xPolygonMiden/miden-base/tree/main/miden-lib/asm/note_scripts) (P2ID, P2IDR and SWAP) that every user can simply create in the Miden Client or by invoking this [function](https://github.com/0xPolygonMiden/miden-base/blob/fa63b26d845f910d12bd5744f34a6e55c08d5cde/miden-lib/src/notes/mod.rs#L15-L66). 

The Note scripts are also the root of a [Miden program MAST](https://0xpolygonmiden.github.io/miden-vm/user_docs/assembly/main.html) which means every function is a commitment to the underlying code. The code cannot change unnoticed to the user because its hash would change. That way it is easy to recognize standardized notes and those which deviate.

### Example note script Pay-to-ID
<details>
  <summary>Want to know more how to ensure a note can only be consumed by a specified account?</summary>

  ### Goal of the P2ID script
  The P2ID script defines a specific target account ID as the only account that can consume the note. Such notes ensure a targeted asset transfer. 

  ### Imports and context
  The P2ID script uses procedures from the account, note and wallet API.
  ```
  use.miden::account
  use.miden::note
  use.miden::contracts::wallets::basic->wallet
  ```
  As discussed in detail in [transaction kernel procedures](../transactions/transaction-procedures.md) certain procedures can only be invoked in certain contexts. The note script is being executed in the note context of the [transaction kernel](../transactions/transaction-kernel.md).

  ### Main script
  The main part of the P2ID script checks if the executing account equals the account defined in the `NoteInputs`. The creator of the note defines the note script and the note inputs separately to ensure usage of the same standardized P2ID script regardless of the target account ID. That way, it is enough to check the script root (see above).

  ```
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

  Every note script starts with the transaction script root on top of the stack. After the `dropw`, the stack is clear. Next, the script loads the note inputs by first getting the pointer to the memory address of the inputs `exec.note::get_inputs`. The `push.0` just ensures that the pointer overrides the newly inserted `0`. Then, `mem_load` loads a `Felt` from the specified memory address and puts it on top of the stack, in that cases the   `target_account_id` defined by the creator of the note. Now, the note invokes `get_id` from the account API using `exec.account::get_id` - which is   possible even in the note context. Because, there are two account IDs on top of the stack now, `assert_eq` fails if the two account IDs   (target_account_id and executing_account_id) are not the same. That means, the script cannot be successfully executed if executed by any other account than the account specified by the note creator using the note inputs.
  
  If execution hasn't failed, the script invokes a helper procedure `exec.add_note_assets_to_account` to add the assets of the note into the executing   account's vault.

  ### Add assets
  This procedure adds the assets held by the note into the account's vault. 

  ```
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
  
          # load the asset and add it to the account
          mem_loadw call.wallet::receive_asset
          # => [ASSET, ptr, end_ptr, ...]
  
          # increment the pointer and compare it to the end_ptr
          movup.4 add.1 dup dup.6 neq
          # => [latch, ptr+1, ASSET, end_ptr, ...]
      end
  
      # clear the stack
      drop dropw drop
  end
  ```

  The procedure starts by calling `exec.note::get_assets` and putting the note's number of assets and the memory pointer of the first asset on top of the stack. Assets are stored in consecutive memory slots, so `dup.1 add` provides the last memory slot. Because [assets](assets.md) are represented by `Words` in Miden Assembly, the procedure pads the stack with four `0`s. Now, if there is at least one asset (checked by `dup dup.6 neq`), the loop starts. It first saves the pointer for later use (`dup movdn.5`), then loads the first asset `mem_loadw` on top of the stack. Now, the procdure calls the a function of the account interface `call.wallet::receive_asset` to put the asset into the account's vault. The note script cannot directly call an account function to add the asset. The account must expose this function in its interface. Lastly, the pointer gets incremented, and if there is a second asset, the loop continues (`movup.4 add.1 dup dup.6 neq`). Finally, when all assets were put into the account's vault, the stack is cleared (`drop dropw drop`).

</details>

## Note storage mode
Similar to accounts, there are two storage modes for notes in Miden. Notes can be stored publicly in the [Notes DB](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#notes-database) with all data visible for everyone. Alternatively, notes can be stored privately committing only the note hash to the Notes DB. 

Every note has a unique single note hash. It is defined as `hash(hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash), vault_hash)`. It is easy to compute the note hash if all the note data is known. To compute a note's hash, we do not need to know the note's `serial_num`. Knowing the hash of the `serial_num` (as well as `script_hash`, `input_hash` and `note_vault`) is also sufficient. We compute the hash of `serial_num` as `hash(serial_num, [0; 4])` to simplify processing within the VM.

Privately stored notes can only be consumed if the note data is known to the consumer. The note data must be provided to the [transaction kernel](../transactions/transaction-kernel.md). That means, there must be some offchain communication to transmit the note's data from the sender to the recipient.

## Note discovery
Note discovery describes the process of Miden Clients finding notes they want to consume. It will always be possible to send note data directly to the target account, and if the note was previously recored on-chain, it can be consumed. However, it is also possible to querry the Miden Operator and request newly recorded relevant notes. This is done via Note Tags. Tags are part of the Note's metadata and are represented by a `Felt`. 

The `SyncState` API requires the Miden Client to provide a `note_tag` value which is used as a filter in the response. 

## Note consumption
As with creation, notes can only be consumed in Miden transactions. If a valid transaction consuming an `InputNote` gets verified by the Miden Node, the note's unique nullifier gets added to the [Nullifier DB](https://0xpolygonmiden.github.io/miden-base/architecture/state.html#nullifier-database) and is therefore consumed. Some notes can be consumed by any account and for some note consumption is restricted.

### Note recipient to restrict note consumption to certain targets
There are several ways to restrict the set of accounts that can consume a specific note. One way is to specifically define the target account ID as done in the P2ID and P2IDR note scripts. Another way is by using the concept of a `RECIPIENT`. Miden defines a `RECIPIENT` as: `hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash)` represented as `Word`. This concept restricts note consumption to those users who know the pre-image data of `RECIPIENT` - which might be a bigger set than a single account.   

During the [transaction prologue](../transactions/transaction-kernel.md) the users needs to provide all the data to compute the note hash. That means, one can create notes that can only be consumed if the `serial_num` and other data is known. This information can be passed on off-chain by the sender to the recipient. 

For public notes, anyone can compute the note hash and the concept of the `RECIPIENT` doesn't make sense. You can see in the standard [SWAP note script](https://github.com/0xPolygonMiden/miden-base/blob/main/miden-lib/asm/note_scripts/SWAP.masm) how `RECIPIENT` is used. Here, using a single hash, is sufficient to ensure that the swapped asset and its note can only be consumed by the defined target.

### Note nullifier to ensure private consumption
The note's nullifier is computed as `hash(serial_num, script_hash, input_hash, vault_hash)`.

This achieves the following properties:
- Every note can be reduced to a single unique nullifier.
- One cannot derive a note's hash from its nullifier.
- To compute the nullifier, one must know all components of the note: `serial_num`, `script_hash`, `input_hash`, and `vault_hash`.

To know a note’s nullifier, one needs to know all details of the note, e.g. the note's serial number. That means if a note is private and the operator stores only the note's hash, only those with the note details know if this note has been consumed already. Zcash first introduced this approach.

<p align="center">
    <img src="../diagrams/architecture/note/Nullifier.png">
</p>
