There are user-facing procedures and kernel procedures. Users don't directly invoke kernel procedures, but instead they invoke them indirectly via account code, note, or transaction scripts. In these cases, kernel procedures are invoked by a `syscall` instruction which always executes in the kernel context.

## User-facing procedures (APIs)

These procedures can be used to create smart contract/account code, note scripts, or account scripts. They basically serve as an API for the underlying kernel procedures. If a procedure can be called in the current context, an `exec` is sufficient. Otherwise the context procedures must be invoked by `call`. Users never need to invoke `syscall` procedures themselves.

!!! tip
    - If capitalized, a variable representing a `word`, e.g., `ACCT_HASH` consists of four `felts`. If lowercase, the variable is represented by a single `felt`.

### Account

To import the account procedures, set `use.miden::account` at the beginning of the file. 

Any procedure that changes the account state must be invoked in the account context and not by note or transaction scripts. All procedures invoke `syscall` to the kernel API and some are restricted by the kernel procedure `exec.authenticate_account_origin`, which fails if the parent context is not the executing account.

| Procedure name            | Stack      | Output       | Context | Description                                                         |
|---------------------------|------------|--------------|---------|---------------------------------------------------------------------|
| `get_id`                  | `[]`       | `[acct_id]`  | account, note | Returns the account id. |
| `get_nonce`               | `[]`       | `[nonce]`    | account, note | Returns the account nonce. |
| `get_initial_hash`        | `[]`       | `[H]`        | account, note | Returns the initial account hash. |
| `get_current_hash`        | `[]`       | `[ACCT_HASH]`| account, note | Computes and returns the account hash from account data stored in memory.
| `incr_nonce`              | `[value]`  | `[]`         | account | Increments the account nonce by the provided `value` which can be at most `2^32 - 1` otherwise the procedure panics. |
| `get_item`                | `[index]`  | `[VALUE]`    | account, note | Gets an item `VALUE` by `index` from the account storage. Panics if the index is out of bounds. |
| `set_item`                | `[index, V']` | `[R', V]` | account | Sets an index/value pair in the account storage. Panics if the index is out of bounds. `R` is the new storage root. |
| `set_code`                | `[CODE_ROOT]`| `[]`       | account | Sets the code (`CODE_ROOT`) of the account the transaction is being executed against. This procedure can only be executed on regular accounts with updatable code. Otherwise, the procedure fails.  |
| `get_balance`             | `[faucet_id]`| `[balance]`| account, note | Returns the `balance` of a fungible asset associated with a `faucet_id`. Panics if the asset is not a fungible asset. |
| `has_non_fungible_asset`  | `[ASSET]`   | `[has_asset]`| account, note | Returns a boolean `has_asset` indicating whether the non-fungible asset is present in the vault. Panics if the `ASSET` is a fungible asset.  |
| `add_asset`               | `[ASSET]`   | `[ASSET']`  | account | Adds the specified asset `ASSET` to the vault. Panics under various conditions. If `ASSET` is a non-fungible asset, then `ASSET'` is the same as `ASSET`. If `ASSET` is a fungible asset, then `ASSET'` is the total fungible asset in the account vault after `ASSET` was added to it. |
| `remove_asset`            | `[ASSET]`   | `[ASSET]`   | account | Remove the specified `ASSET` from the vault. Panics under various conditions.  |
| `get_vault_commitment`    | `[]`        | `[COM]`     | account, note | Returns a commitment `COM` to the account vault.  |

### Note

To import the note procedures, set `use.miden::note` at the beginning of the file. All procedures are restricted to the note context.

| Procedure name           | Inputs              | Outputs               | Context | Description                                                                                                                         |
|--------------------------|---------------------|-----------------------|---------|-------------------------------------------------------------------------------------------------------------------------------------|
| `get_assets`             | `[dest_ptr]`        | `[num_assets, dest_ptr]` | note | Writes the assets of the currently executing note into memory starting at the specified address `dest_ptr `. is the memory address to write the assets. `num_assets` is the number of assets in the currently executing note. |
| `get_inputs`             | `[dest_ptr]`        | `[dest_ptr]`            | note | Writes the inputs of the currently executed note into memory starting at the specified address, `dest_ptr`. |
| `get_sender`             | `[]`                | `[sender]`             | note | Returns the `sender` of the note currently being processed. Panics if a note is not being processed.  |


### Tx
To import the transaction procedures set `use.miden::tx` at the beginning of the file. Only the `create_note` procedure is restricted to the account context.

| Procedure name           | Inputs           | Outputs     | Context | Description                                                                                                                                                                  |
|--------------------------|------------------|-------------|---------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `get_block_number`       | `[]`             | `[num]`     | account, note | Returns the block number `num` of the last known block at the time of transaction execution. |
| `get_block_hash`         | `[]`             | `[H]`       |  account, note | Returns the block hash `H` of the last known block at the time of transaction execution. |
| `get_input_notes_hash`   | `[]`             | `[COM]`     |  account, note | Returns the input notes hash `COM`. This is computed as a sequential hash of (nullifier, script_root) tuples over all input notes.  |
| `get_output_notes_hash`  | `[0, 0, 0, 0]`   | `[COM]`     |  account, note | Returns the output notes hash `COM`. This is computed as a sequential hash of (note_hash, note_metadata) tuples over all output notes.  |
| `create_note`            | `[ASSET, tag, RECIPIENT]` | `[ptr]` | account | Creates a new note and returns a pointer to the memory address at which the note is stored. `ASSET` is the asset to be included in the note. `tag` is the tag to be included in the note. `RECIPIENT` is the recipient of the note. `ptr` is the pointer to the memory address at which the note is stored. |


### Asset
To import the asset procedures set `use.miden::asset` at the beginning of the file. These procedures can only be called by faucet accounts.

| Procedure name               | Stack               | Output    | Context | Description                                                                                                                                                 |
|------------------------------|---------------------|-----------|---------|-------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `build_fungible_asset`       | `[faucet_id, amount]` | `[ASSET]` | faucet | Builds a fungible asset `ASSET` for the specified fungible faucet `faucet_id`, and `amount` of asset to create. |
| `create_fungible_asset`      | `[amount]`          | `[ASSET]` | faucet | Creates a fungible asset `ASSET` for the faucet the transaction is being executed against and `amount` of the asset to create.  |
| `build_non_fungible_asset`   | `[faucet_id, DATA_HASH]` | `[ASSET]` | faucet | Builds a non-fungible asset `ASSET` for the specified non-fungible faucet where `faucet_id` is the faucet to create the asset for and `DATA_HASH` is the data hash of the non-fungible asset to build. |
| `create_non_fungible_asset`  | `[DATA_HASH]`        | `[ASSET]` | faucet | Creates a non-fungible asset `ASSET` for the faucet the transaction is being executed against. `DATA_HASH` is the data hash of the non-fungible asset to create.  |

### Faucet

To import the faucet procedures, set `use.miden::faucet` at the beginning of the file.

| Procedure name           | Stack      | Outputs           | Context | Description                 |
|--------------------------|------------|-------------------|---------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `mint`                   | `[ASSET]`  | `[ASSET]`         | faucet | Mint an asset `ASSET` from the faucet the transaction is being executed against. Panics under various conditions.  |
| `burn`                   | `[ASSET]`  | `[ASSET]`         | faucet | Burn an asset `ASSET` from the faucet the transaction is being executed against. Panics under various conditions.  |
| `get_total_issuance`     | `[]`       | `[total_issuance]`| faucet | Returns the `total_issuance` of the fungible faucet the transaction is being executed against. Panics if the transaction is not being executed against a fungible faucet. |


