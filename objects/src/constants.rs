/// Depth of the account database tree.
pub const ACCOUNT_TREE_DEPTH: u8 = 64;

/// The depth of the Merkle tree used to commit to notes produced in a block.
pub const NOTE_TREE_DEPTH: u8 = 20;

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
///
/// A single note uses two leaves in the tree. The even leaf is used to store the note's id, the
/// odd leaf is used to store the note's metadata.
pub const BATCH_OUTPUT_NOTES_TREE_DEPTH: u8 = 13;

/// The maximum number of notes that can be created in a single batch.
///
/// Because the tree used in a batch has fixed depth, and each note takes two leaves, the maximum
/// number of notes is the number of leaves in the tree.
pub const MAX_NOTES_PER_BATCH: usize = 2_usize.pow((BATCH_OUTPUT_NOTES_TREE_DEPTH - 1) as u32);

// BLOCK
// ================================================================================================

/// The depth of the Sparse Merkle Tree used to store a batch's note tree.
///
/// This value can be interpreted as:
///
/// - The depth of a tree with the leaves set to a batch output note tree root.
/// - The level at which the batches create note trees are merged, creating a new tree with this
///   many additional new levels.
pub const BLOCK_OUTPUT_NOTES_BATCH_TREE_DEPTH: u8 = 8;

/// The final depth of the Sparse Merkle Tree used to store all notes created in a block.
pub const BLOCK_OUTPUT_NOTES_TREE_DEPTH: u8 =
    BATCH_OUTPUT_NOTES_TREE_DEPTH + BLOCK_OUTPUT_NOTES_BATCH_TREE_DEPTH;

/// Maximum number of batches that can be inserted into a single block.
pub const MAX_BATCHES_PER_BLOCK: usize = 2_usize.pow(BLOCK_OUTPUT_NOTES_BATCH_TREE_DEPTH as u32);

/// Maximum number of output notes that can be created in a single block.
pub const MAX_OUTPUT_NOTES_PER_BLOCK: usize = MAX_NOTES_PER_BATCH * MAX_BATCHES_PER_BLOCK;

/// The block height of the genesis block
pub const GENESIS_BLOCK: u32 = 0;
