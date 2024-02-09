# Miden node RPC
The **RPC** is an externally-facing component through which clients can interact with the node. It receives client requests (e.g., to synchronize with the latest state of the chain, or to submit transactions), performs basic validation, 
and forwards the requests to the appropriate components.

**RPC** is one of components of the Miden node.

## Architecture
`TODO`

## API
The **RPC** serves connections using the [gRPC protocol](https://grpc.io) on a port, set in the previously mentioned configuration file. 
Here is a brief description of supported methods.

### CheckNullifiers
Gets a list of proofs for given nullifier hashes, each proof as Sparse Merkle Trees 

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

### SubmitProvenTransaction
Submits proven transaction to the Miden network.

**Parameters**

* `transaction`: `bytes` - transaction encoded using Miden's native format.

**Returns**

This method doesn't return any data.
