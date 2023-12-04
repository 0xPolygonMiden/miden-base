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
/// Fungible faucet: The faucet data consists of [ZERO, ZERO, ZERO, total_issuance]
/// Non-fungible faucet: The faucet data consists of SMT root containing minted non-fungible assets.
pub const FAUCET_STORAGE_DATA_SLOT: StorageSlot = 255;

// BOOKKEEPING
// ------------------------------------------------------------------------------------------------

/// The memory address at which the transaction vault root is stored.
pub const TX_VAULT_ROOT_PTR: MemoryAddress = 0;

/// The memory address at which a pointer to the consumed note being executed is stored.
pub const CURRENT_CONSUMED_NOTE_PTR: MemoryAddress = 1;

/// The memory address at which the number of created notes is stored.
pub const NUM_CREATED_NOTES_PTR: MemoryAddress = 2;

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

/// The memory address at which the global nullifier commitment is stored.
pub const NULLIFIER_COM_PTR: MemoryAddress = 103;

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

/// The memory address at which the batch root is stored
pub const BATCH_ROOT_PTR: MemoryAddress = 204;

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

// NOTES DATA
// ------------------------------------------------------------------------------------------------

/// The maximum number of assets that can be stored in a single note.
pub const MAX_ASSETS_PER_NOTE: u32 = 256;

/// The size of the memory segment allocated to each note
pub const NOTE_MEM_SIZE: MemoryAddress = 1024;

// CONSUMED NOTES DATA
// ------------------------------------------------------------------------------------------------

/// The memory address at which the consumed note section begins.
pub const CONSUMED_NOTE_SECTION_OFFSET: MemoryOffset = 1000;

/// The memory address at which the number of consumed notes is stored.
pub const CONSUMED_NOTE_NUM_PTR: MemoryAddress = 1000;

/// The offsets at which data of a consumed note is stored relative to the start of its data segment.
pub const CONSUMED_NOTE_HASH_OFFSET: MemoryOffset = 0;
pub const CONSUMED_NOTE_SERIAL_NUM_OFFSET: MemoryOffset = 1;
pub const CONSUMED_NOTE_SCRIPT_ROOT_OFFSET: MemoryOffset = 2;
pub const CONSUMED_NOTE_INPUTS_HASH_OFFSET: MemoryOffset = 3;
pub const CONSUMED_NOTE_VAULT_ROOT_OFFSET: MemoryOffset = 4;
pub const CONSUMED_NOTE_METADATA_OFFSET: MemoryOffset = 5;
pub const CONSUMED_NOTE_ASSETS_OFFSET: MemoryOffset = 6;

/// The maximum number of consumed notes that can be processed in a single transaction.
pub const MAX_NUM_CONSUMED_NOTES: u32 = 1023;

// CREATED NOTES DATA
// ------------------------------------------------------------------------------------------------

/// The size of the core created note data segment.
pub const CREATED_NOTE_CORE_DATA_SIZE: MemSize = 4;

/// The memory address at which the created notes section begins.
pub const CREATED_NOTE_SECTION_OFFSET: MemoryOffset = 10000;

/// The offsets at which data of a created note is stored relative to the start of its data segment.
pub const CREATED_NOTE_HASH_OFFSET: MemoryOffset = 0;
pub const CREATED_NOTE_METADATA_OFFSET: MemoryOffset = 1;
pub const CREATED_NOTE_RECIPIENT_OFFSET: MemoryOffset = 2;
pub const CREATED_NOTE_VAULT_HASH_OFFSET: MemoryOffset = 3;
pub const CREATED_NOTE_ASSETS_OFFSET: MemoryOffset = 4;

/// The maximum number of created notes that can be prodcued in a single transaction.
pub const MAX_NUM_CREATED_NOTES: u32 = 4096;
