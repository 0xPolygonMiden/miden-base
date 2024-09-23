/// Depth of the account database tree.
pub const ACCOUNT_TREE_DEPTH: u8 = 64;

/// The maximum number of assets that can be stored in a single note.
pub const MAX_ASSETS_PER_NOTE: usize = 256;

/// The maximum number of inputs that can accompany a single note.
///
/// The value is set to 128 so that it can be represented using as a single byte while being
/// evenly divisible by 8.
pub const MAX_INPUTS_PER_NOTE: usize = 128;

/// The maximum number of notes that can be consumed by a single transaction.
pub const MAX_INPUT_NOTES_PER_TX: usize = 1024;

/// The maximum number of new notes created by a single transaction.
pub const MAX_OUTPUT_NOTES_PER_TX: usize = MAX_INPUT_NOTES_PER_TX;

/// The minimum proof security level used by the Miden prover & verifier.
pub const MIN_PROOF_SECURITY_LEVEL: u32 = 96;

// TRANSACTION BATCH
// ================================================================================================

/// The depth of the Sparse Merkle Tree used to store output notes in a single batch.
pub const BATCH_NOTE_TREE_DEPTH: u8 = 10;

/// The maximum number of notes that can be created in a single batch.
pub const MAX_NOTES_PER_BATCH: usize = 1 << BATCH_NOTE_TREE_DEPTH;
const _: () = assert!(MAX_NOTES_PER_BATCH >= MAX_OUTPUT_NOTES_PER_TX);

/// The maximum number of input notes that can be consumed in a single batch.
pub const MAX_INPUT_NOTES_PER_BATCH: usize = MAX_NOTES_PER_BATCH;
const _: () = assert!(MAX_INPUT_NOTES_PER_BATCH >= MAX_INPUT_NOTES_PER_TX);

/// The maximum number of transaction in a single batch.
pub const MAX_TRANSACTIONS_PER_BATCH: usize = MAX_NOTES_PER_BATCH;

// BLOCK
// ================================================================================================

/// The final depth of the Sparse Merkle Tree used to store all notes created in a block.
pub const BLOCK_NOTE_TREE_DEPTH: u8 = 16;

/// Maximum number of batches that can be inserted into a single block.
pub const MAX_BATCHES_PER_BLOCK: usize = 1 << (BLOCK_NOTE_TREE_DEPTH - BATCH_NOTE_TREE_DEPTH);

/// Maximum number of output notes that can be created in a single block.
pub const MAX_NOTES_PER_BLOCK: usize = MAX_NOTES_PER_BATCH * MAX_BATCHES_PER_BLOCK;
const _: () = assert!(MAX_NOTES_PER_BLOCK >= MAX_NOTES_PER_BATCH);

pub const MAX_INPUT_NOTES_PER_BLOCK: usize = MAX_NOTES_PER_BLOCK;

/// The maximum number of transaction in a single block.
pub const MAX_TRANSACTIONS_PER_BLOCK: usize = MAX_TRANSACTIONS_PER_BATCH * MAX_BATCHES_PER_BLOCK;
const _: () = assert!(MAX_TRANSACTIONS_PER_BLOCK >= MAX_TRANSACTIONS_PER_BATCH);
const _: () = assert!(MAX_TRANSACTIONS_PER_BLOCK >= MAX_BATCHES_PER_BLOCK);

/// The block height of the genesis block
pub const GENESIS_BLOCK: u32 = 0;
