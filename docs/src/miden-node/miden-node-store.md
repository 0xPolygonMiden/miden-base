# Miden node store
The **Store** maintains the state of the chain. It serves as the "source of truth" for the chain - i.e., if it is not in 
the store, the node does not consider it to be part of the chain. 
**Store** is one of components of the Miden node.

## Architecture
`TODO`

## API
The **Store** serves connections using the [gRPC protocol](https://grpc.io) on a port, set in the previously mentioned configuration file. The API cannot directly be called by the Miden Client.

Here is a brief description of supported methods.

### ApplyBlock

Applies changes of a new block to the DB and in-memory data structures.

**Parameters**

* `block`: `BlockHeader` – block header ([src](https://github.com/0xPolygonMiden/miden-node/blob/main/proto/proto/block_header.proto)).
* `accounts`: `[AccountUpdate]` – a list of account updates.
* `nullifiers`: `[Digest]` – a list of nullifier hashes.
* `notes`: `[NoteCreated]` – a list of notes created.

**Returns**

This method doesn't return any data.

### CheckNullifiers

Get a list of proofs for given nullifier hashes, each proof as Sparse Merkle Trees

**Parameters:**

* `nullifiers`: `[Digest]` – array of nullifier hashes.

**Returns:**

* `proofs`: `[NullifierProof]` – array of nullifier proofs, positions correspond to the ones in request.

### GetBlockHeaderByNumber

Retrieves block header by given block number.

**Parameters**

* `block_num`: `uint32` *(optional)* – the block number of the target block. If not provided, the latest known block will be returned.

**Returns:**

* `block_header`: `BlockHeader` – block header.

### GetBlockInputs

Returns data needed by the block producer to construct and prove the next block.

**Parameters**

* `account_ids`: `[AccountId]` – array of account IDs. 
* `nullifiers`: `[Digest]` – array of nullifier hashes (not currently in use).

**Returns**

* `block_header`: `[BlockHeader]` – the latest block header.
* `mmr_peaks`: `[Digest]` – peaks of the above block's mmr, The `forest` value is equal to the block number.
* `account_states`: `[AccountBlockInputRecord]` – the hashes of the requested accounts and their authentication paths.
* `nullifiers`: `[NullifierBlockInputRecord]` – the requested nullifiers and their authentication paths.

### GetTransactionInputs

Returns the data needed by the block producer to check validity of an incoming transaction. 

**Parameters**

* `account_id`: `AccountId` – ID of the account against which a transaction is executed.
* `nullifiers`: `[Digest]` – array of nullifiers for all notes consumed by a transaction.

**Returns**

* `account_state`: `AccountTransactionInputRecord` – account's descriptors. 
* `nullifiers`: `[NullifierTransactionInputRecord]` – the block numbers at which corresponding nullifiers have been consumed, zero if not consumed.

### SyncState
Returns info which can be used by the client to sync up to the latest state of the chain
for the objects (accounts, notes, nullifiers) the client is interested in.

This request returns the next block containing requested data. It also returns `chain_tip` which is the latest block number in the chain.
Client is expected to repeat these requests in a loop until `response.block_header.block_num == response.chain_tip`, at which point the client is fully synchronized with the chain.

Each request also returns info about new notes, nullifiers etc. created. It also returns Chain MMR delta that can be used to update the state of Chain MMR.
This includes both chain MMR peaks and chain MMR nodes.

For preserving some degree of privacy, note tags and nullifiers filters contain only high part of hashes. Thus, returned data
contains excessive notes and nullifiers, client can make additional filtering of that data on its side.

**Parameters**

* `block_num`: `uint32` – send updates to the client starting at this block.
* `account_ids`: `[AccountId]` – accounts filter.
* `note_tags`: `[uint32]` – note tags filter. Corresponds to the high 16 bits of the real values.
* `nullifiers`: `[uint32]` – nullifiers filter. Corresponds to the high 16 bits of the real values.

**Returns**

* `chain_tip`: `uint32` – number of the latest block in the chain.
* `block_header`: `BlockHeader` – block header of the block with the first note matching the specified criteria.
* `mmr_delta`: `MmrDelta` – data needed to update the partial MMR from `block_num` to `block_header.block_num`.
* `block_path`: `MerklePath` – Merkle path in the updated chain MMR to the block at `block_header.block_num`.
* `accounts`: `[AccountHashUpdate]` – a list of account hashes updated after `block_num` but not after `block_header.block_num`.
* `notes`: `[NoteSyncRecord]` – a list of all notes together with the Merkle paths from `block_header.note_root`.
* `nullifiers`: `[NullifierUpdate]` – a list of nullifiers created between `block_num` and `block_header.block_num`.

## Methods for testing purposes

### ListNullifiers

Lists all nullifiers of the current chain.

**Parameters**

This request doesn't have any parameters.

**Returns**

* `nullifiers`: `[NullifierLeaf]` – lists of all nullifiers of the current chain. 

### ListAccounts

Lists all accounts of the current chain.

**Parameters**

This request doesn't have any parameters.

**Returns**

* `accounts`: `[AccountInfo]` – list of all accounts of the current chain.

### ListNotes

Lists all notes of the current chain.

**Parameters**

This request doesn't have any parameters.

**Returns**

* `notes`: `[Note]` – list of all notes of the current chain.
