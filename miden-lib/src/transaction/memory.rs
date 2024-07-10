// TYPE ALIASES
// ================================================================================================

pub type MemoryAddress = u32;
pub type MemoryOffset = u32;
pub type DataIndex = usize;
pub type MemSize = usize;
pub type StorageSlot = u8;

// PUBLIC CONSTANTS
// ================================================================================================

// RESERVED ACCOUNT STORAGE SLOTS
// ------------------------------------------------------------------------------------------------

/// The account storage slot at which faucet data is stored.
/// Fungible faucet: The faucet data consists of [0, 0, 0, total_issuance]
/// Non-fungible faucet: The faucet data consists of SMT root containing minted non-fungible assets.
pub const FAUCET_STORAGE_DATA_SLOT: StorageSlot = 254;

/// The account storage slot at which the slot types commitment is stored.
pub const SLOT_TYPES_COMMITMENT_STORAGE_SLOT: StorageSlot = 255;

// BOOKKEEPING
// ------------------------------------------------------------------------------------------------

/// The memory address at which the transaction vault root is stored.
pub const TX_VAULT_ROOT_PTR: MemoryAddress = 0;

/// The memory address at which a pointer to the input note being executed is stored.
pub const CURRENT_INPUT_NOTE_PTR: MemoryAddress = 1;

/// The memory address at which the number of output notes is stored.
pub const NUM_OUTPUT_NOTES_PTR: MemoryAddress = 2;

/// The memory address at which the input vault root is stored
pub const INPUT_VAULT_ROOT_PTR: MemoryAddress = 3;

/// The memory address at which the output vault root is stored
pub const OUTPUT_VAULT_ROOT_PTR: MemoryAddress = 4;

// GLOBAL INPUTS
// ------------------------------------------------------------------------------------------------

/// The memory address at which the global inputs section begins.
pub const GLOBAL_INPUTS_SECTION_OFFSET: MemoryOffset = 100;

/// The memory address at which the latest known block hash is stored.
pub const BLK_HASH_PTR: MemoryAddress = 100;

/// The memory address at which the account id is stored.
pub const ACCT_ID_PTR: MemoryAddress = 101;

/// The memory address at which the initial account hash is stored.
pub const INIT_ACCT_HASH_PTR: MemoryAddress = 102;

/// The memory address at which the input notes commitment is stored.
pub const INPUT_NOTES_COMMITMENT_PTR: MemoryAddress = 103;

/// The memory address at which the initial nonce is stored.
pub const INIT_NONCE_PTR: MemoryAddress = 104;

/// The memory address at which the transaction script mast root is store
pub const TX_SCRIPT_ROOT_PTR: MemoryAddress = 105;

// BLOCK DATA
// ------------------------------------------------------------------------------------------------

/// The memory address at which the block data section begins
pub const BLOCK_DATA_SECTION_OFFSET: MemoryOffset = 200;

/// The memory address at which the previous block hash is stored
pub const PREV_BLOCK_HASH_PTR: MemoryAddress = 200;

/// The memory address at which the chain root is stored
pub const CHAIN_ROOT_PTR: MemoryAddress = 201;

/// The memory address at which the state root is stored
pub const ACCT_DB_ROOT_PTR: MemoryAddress = 202;

/// The memory address at which the nullifier db root is store
pub const NULLIFIER_DB_ROOT_PTR: MemoryAddress = 203;

/// The memory address at which the TX hash is stored
pub const TX_HASH_PTR: MemoryAddress = 204;

/// The memory address at which the proof hash is stored
pub const PROOF_HASH_PTR: MemoryAddress = 205;

/// The memory address at which the block number is stored
pub const BLOCK_METADATA_PTR: MemoryAddress = 206;

/// The index of the block number within the block metadata
pub const BLOCK_NUMBER_IDX: DataIndex = 0;

/// The index of the protocol version within the block metadata
pub const PROTOCOL_VERSION_IDX: DataIndex = 1;

/// The index of the timestamp within the block metadata
pub const TIMESTAMP_IDX: DataIndex = 2;

/// The memory address at which the note root is stored
pub const NOTE_ROOT_PTR: MemoryAddress = 207;

// CHAIN DATA
// ------------------------------------------------------------------------------------------------

/// The memory address at which the chain data section begins
pub const CHAIN_MMR_PTR: MemoryAddress = 300;

/// The memory address at which the total number of leaves in the chain MMR is stored
pub const CHAIN_MMR_NUM_LEAVES_PTR: MemoryAddress = 300;

/// The memory address at which the chain mmr peaks are stored
pub const CHAIN_MMR_PEAKS_PTR: MemoryAddress = 301;

// ACCOUNT DATA
// ------------------------------------------------------------------------------------------------

/// The size of the memory segment allocated to core account data (excluding new code root)
pub const ACCT_DATA_MEM_SIZE: MemSize = 4;

/// The memory address at which the account data section begins
pub const ACCT_DATA_SECTION_OFFSET: MemoryOffset = 400;

/// The offset at which the account id and nonce is stored relative to the start of the account
/// data segment.
pub const ACCT_ID_AND_NONCE_OFFSET: MemoryOffset = 0;

/// The index of the account id within the account id and nonce data.
pub const ACCT_ID_IDX: DataIndex = 0;

/// The index of the account nonce within the account id and nonce data.
pub const ACCT_NONCE_IDX: DataIndex = 3;

/// The memory address at which the account id and nonce is stored.
/// The account id is stored in the first element.
/// The account nonce is stored in the fourth element.
pub const ACCT_ID_AND_NONCE_PTR: MemoryAddress =
    ACCT_DATA_SECTION_OFFSET + ACCT_ID_AND_NONCE_OFFSET;

/// The offset at which the account vault root is stored relative to the start of the account
/// data segment.
pub const ACCT_VAULT_ROOT_OFFSET: MemoryOffset = 1;

/// The memory address at which the account vault root is stored.
pub const ACCT_VAULT_ROOT_PTR: MemoryAddress = ACCT_DATA_SECTION_OFFSET + ACCT_VAULT_ROOT_OFFSET;

/// The offset at which the account storage root is stored relative to the start of the account
/// data segment.
pub const ACCT_STORAGE_ROOT_OFFSET: MemoryOffset = 2;

/// The memory address at which the account storage root is stored.
pub const ACCT_STORAGE_ROOT_PTR: MemoryAddress =
    ACCT_DATA_SECTION_OFFSET + ACCT_STORAGE_ROOT_OFFSET;

/// The offset at which the account code root is stored relative to the start of the account
/// data segment.
pub const ACCT_CODE_ROOT_OFFSET: MemoryOffset = 3;

/// The memory address at which the account code root is stored.
pub const ACCT_CODE_ROOT_PTR: MemoryAddress = ACCT_DATA_SECTION_OFFSET + ACCT_CODE_ROOT_OFFSET;

/// The offset at which the accounts new code root is stored relative to the start of the account
/// data segment.
pub const ACCT_NEW_CODE_ROOT_OFFSET: MemoryOffset = 4;

/// The memory address at which the new account code root is stored
pub const ACCT_NEW_CODE_ROOT_PTR: MemoryAddress =
    ACCT_DATA_SECTION_OFFSET + ACCT_NEW_CODE_ROOT_OFFSET;

/// The memory address at which the account storage slot type data beings
pub const ACCT_STORAGE_SLOT_TYPE_DATA_OFFSET: MemoryAddress = 405;

// NOTES DATA
// ================================================================================================

/// The size of the memory segment allocated to each note.
pub const NOTE_MEM_SIZE: MemoryAddress = 512;

// INPUT NOTES DATA
// ------------------------------------------------------------------------------------------------
// Inputs note section contains data of all notes consumed by a transaction. The section starts at
// memory offset 1_048_576 with a word containing the total number of input notes and is followed
// by note nullifiers and note data like so:
//
// ┌─────────┬───────────┬───────────┬─────┬───────────┬─────────┬────────┬────────┬─────┬────────┐
// │   NUM   │  NOTE 0   │  NOTE 1   │ ... │  NOTE n   │ PADDING │ NOTE 0 │ NOTE 1 │ ... │ NOTE n │
// │  NOTES  │ NULLIFIER │ NULLIFIER │     │ NULLIFIER │         │  DATA  │  DATA  │     │  DATA  │
// └─────────┴───────────┴───────────┴─────┴───────────┴─────────┴────────┴────────┴─────┴────────┘
//  1_048_576  1_048_577   1_048_578        1_048_576+n      1_064_960   +512    +1024  +512n
//
// Each nullifier occupies a single word. A data section for each note consists of exactly 512
// words and is laid out like so:
//
// ┌──────┬────────┬────────┬────────┬────────┬──────┬───────┬────────┬───────┬─────┬───────┬─────────┬
// │ NOTE │ SERIAL │ SCRIPT │ INPUTS │ ASSETS │ META │ NOTE  │   NUM  │ ASSET │ ... │ ASSET │ PADDING │
// │  ID  │  NUM   │  ROOT  │  HASH  │  HASH  │ DATA │ ARGS  │ ASSETS │   0   │     │   n   │         │
// ├──────┼────────┼────────┼────────┼────────┼──────┼───────┼────────┼───────┼─────┼───────┼─────────┤
//    0        1       2        3        4       5       6       7      8 + n
//
// - NUM_ASSETS is encoded [num_assets, 0, 0, 0].
// - INPUTS_HASH is the key to look up note inputs in the advice map.
// - ASSETS_HASH is the key to look up note assets in the advice map.

/// The memory address at which the input note section begins.
pub const INPUT_NOTE_SECTION_OFFSET: MemoryOffset = 1_048_576;

/// The memory address at which the input note data section begins.
pub const INPUT_NOTE_DATA_SECTION_OFFSET: MemoryAddress = 1_064_960;

/// The memory address at which the number of input notes is stored.
pub const NUM_INPUT_NOTES_PTR: MemoryAddress = INPUT_NOTE_SECTION_OFFSET;

/// The offsets at which data of a input note is stored relative to the start of its data segment.
pub const INPUT_NOTE_ID_OFFSET: MemoryOffset = 0;
pub const INPUT_NOTE_SERIAL_NUM_OFFSET: MemoryOffset = 1;
pub const INPUT_NOTE_SCRIPT_ROOT_OFFSET: MemoryOffset = 2;
pub const INPUT_NOTE_INPUTS_HASH_OFFSET: MemoryOffset = 3;
pub const INPUT_NOTE_ASSETS_HASH_OFFSET: MemoryOffset = 4;
pub const INPUT_NOTE_METADATA_OFFSET: MemoryOffset = 5;
pub const INPUT_NOTE_ARGS_OFFSET: MemoryOffset = 6;
pub const INPUT_NOTE_NUM_ASSETS_OFFSET: MemoryOffset = 7;
pub const INPUT_NOTE_ASSETS_OFFSET: MemoryOffset = 8;

// OUTPUT NOTES DATA
// ------------------------------------------------------------------------------------------------
// Output notes section contains data of all notes produced by a transaction. The section starts at
// memory offset 4_194_304 with each note data laid out one after another in 512 word increments.
//
//    ┌─────────────┬─────────────┬───────────────┬─────────────┐
//    │ NOTE 0 DATA │ NOTE 1 DATA │      ...      │ NOTE n DATA │
//    └─────────────┴─────────────┴───────────────┴─────────────┘
// 4_194_304      +512          +1024           +512n
//
// The total number of output notes for a transaction is stored in the bookkeeping section of the
// memory. Data section of each note is laid out like so:
//
// ┌─────────┬──────────┬───────────┬─────────────┬────────────┬─────────┬─────┬─────────┬─────────┐
// │ NOTE ID │ METADATA │ RECIPIENT │ ASSETS HASH │ NUM ASSETS │ ASSET 0 │ ... │ ASSET n │ PADDING │
// ├─────────┼──────────┼───────────┼─────────────┼────────────┼─────────┼─────┼─────────┼─────────┤
//      0          1          2            3            4           5             5 + n
//
// Even though NUM_ASSETS takes up a while word, the actual value of this variable is stored in the
// first element of the word.

/// The memory address at which the output notes section begins.
pub const OUTPUT_NOTE_SECTION_OFFSET: MemoryOffset = 4_194_304;

/// The size of the core output note data segment.
pub const OUTPUT_NOTE_CORE_DATA_SIZE: MemSize = 4;

/// The offsets at which data of a output note is stored relative to the start of its data segment.
pub const OUTPUT_NOTE_ID_OFFSET: MemoryOffset = 0;
pub const OUTPUT_NOTE_METADATA_OFFSET: MemoryOffset = 1;
pub const OUTPUT_NOTE_RECIPIENT_OFFSET: MemoryOffset = 2;
pub const OUTPUT_NOTE_ASSET_HASH_OFFSET: MemoryOffset = 3;
pub const OUTPUT_NOTE_NUM_ASSETS_OFFSET: MemoryOffset = 4;
pub const OUTPUT_NOTE_ASSETS_OFFSET: MemoryOffset = 5;
