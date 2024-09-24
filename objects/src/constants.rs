/// Depth of the account database tree.
pub const ACCOUNT_TREE_DEPTH: u8 = 64;

/// The maximum allowed size of an account delta is 32 KiB.
pub const ACCOUNT_DELTA_MAX_SIZE: u16 = 2u16.pow(15);

/// The maximum number of assets that can be stored in a single note.
pub const MAX_ASSETS_PER_NOTE: usize = 256;

/// The maximum number of inputs that can accompany a single note.
///
/// The value is set to 128 so that it can be represented using as a single byte while being
/// evenly divisible by 8.
pub const MAX_INPUTS_PER_NOTE: usize = 128;

/// The maximum number of notes that can be consumed by a single transaction.
pub const MAX_INPUT_NOTES_PER_TX: usize = 1023;

/// The maximum number of new notes created by a single transaction.
pub const MAX_OUTPUT_NOTES_PER_TX: usize = 4096;

/// The minimum proof security level used by the Miden prover & verifier.
pub const MIN_PROOF_SECURITY_LEVEL: u32 = 96;

// TRANSACTION BATCH
// ================================================================================================

/// The depth of the Sparse Merkle Tree used to store output notes in a single batch.
pub const BATCH_NOTE_TREE_DEPTH: u8 = 10;

/// The maximum number of notes that can be created in a single batch.
pub const MAX_NOTES_PER_BATCH: usize = 1 << BATCH_NOTE_TREE_DEPTH;

// BLOCK
// ================================================================================================

/// The final depth of the Sparse Merkle Tree used to store all notes created in a block.
pub const BLOCK_NOTE_TREE_DEPTH: u8 = 16;

/// Maximum number of batches that can be inserted into a single block.
pub const MAX_BATCHES_PER_BLOCK: usize = 1 << (BLOCK_NOTE_TREE_DEPTH - BATCH_NOTE_TREE_DEPTH);

/// Maximum number of output notes that can be created in a single block.
pub const MAX_NOTES_PER_BLOCK: usize = MAX_NOTES_PER_BATCH * MAX_BATCHES_PER_BLOCK;

/// The block height of the genesis block
pub const GENESIS_BLOCK: u32 = 0;
