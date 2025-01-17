// TYPE ALIASES
// ================================================================================================

pub type MemoryAddress = u32;
pub type MemoryOffset = u32;
pub type DataIndex = usize;
pub type MemSize = usize;
pub type StorageSlot = u8;

// PUBLIC CONSTANTS
// ================================================================================================

// General layout
//
// Here the "end address" is the last memory address occupied by the current data
// 
// | Section           | Start address | End address | Start address (words) |  End address (words) |
// | ----------------- | :-----------: | :---------: | :--------------------:| :-------------------:|
// | Bookkeeping       | 0             | 31          | 0                     | 7                    |
// | Global inputs     | 400           | 423         | 100                   | 105                  |
// | Block header      | 800           | 835         | 200                   | 208                  |
// | Chain MMR         | 1_200         | 1_331?      | 300                   | 332?                 |
// | Kernel data       | 1_600         | 1_739       | 400                   | 434                  |
// | Accounts data     | 8_192         | 532_479     | 2048                  | 133_119              | 64 foreign accounts max
// | Input notes       | 4_194_304     | ?           | 1_048_576             | ?                    |
// | Output notes      | 16_777_216    | ?           | 4_194_304             | ?                    |

// Relative layout of one account
//
// Here the "end pointer" is the last memory pointer occupied by the current data
//
// | Section           | Start pointer | End pointer | Start pointer (words) |  End pointer (words) |
// | ----------------- | :-----------: | :---------: | :--------------------:| :-------------------:|
// | Id and nonce      | 0             | 3           | 0                     | 0                    |
// | Vault root        | 4             | 7           | 1                     | 1                    |
// | Storage root      | 8             | 11          | 2                     | 2                    |
// | Code root         | 12            | 15          | 3                     | 3                    |
// | Padding           | 16            | 27          | 4                     | 6                    |
// | Num procedures    | 28            | 31          | 7                     | 7                    |
// | Procedures info   | 32            | 2_079       | 8                     | 519                  |
// | Padding           | 2_080         | 2_083       | 520                   | 520                  |
// | Num storage slots | 2_084         | 2_087       | 521                   | 521                  |
// | Storage slot info | 2_088         | 4_127       | 522                   | 1031                 |
// | Padding           | 4_128         | 8_191       | 1032                  | 2047                 |

// RESERVED ACCOUNT STORAGE SLOTS
// ------------------------------------------------------------------------------------------------

/// The account storage slot at which faucet data is stored.
///
/// - Fungible faucet: The faucet data consists of [0, 0, 0, total_issuance].
/// - Non-fungible faucet: The faucet data consists of SMT root containing minted non-fungible
///   assets.
pub const FAUCET_STORAGE_DATA_SLOT: StorageSlot = 0;

// BOOKKEEPING
// ------------------------------------------------------------------------------------------------

/// The memory address at which the transaction vault root is stored.
pub const TX_VAULT_ROOT_PTR: MemoryAddress = 0;

/// The memory address at which a pointer to the input note being executed is stored.
pub const CURRENT_INPUT_NOTE_PTR: MemoryAddress = 4;

/// The memory address at which the number of output notes is stored.
pub const NUM_OUTPUT_NOTES_PTR: MemoryAddress = 8;

/// The memory address at which the input vault root is stored
pub const INPUT_VAULT_ROOT_PTR: MemoryAddress = 12;

/// The memory address at which the output vault root is stored
pub const OUTPUT_VAULT_ROOT_PTR: MemoryAddress = 16;

/// The memory address at which the pointer to the data of the currently accessing account is stored
pub const CURRENT_ACCOUNT_DATA_PTR: MemoryAddress = 20;

/// The memory address at which the native account's new code commitment is stored.
pub const NEW_CODE_ROOT_PTR: MemoryAddress = 24;

/// The memory address at which the transaction expiration block number is stored.
pub const TX_EXPIRATION_BLOCK_NUM_PTR: MemoryAddress = 28;

// GLOBAL INPUTS
// ------------------------------------------------------------------------------------------------

/// The memory address at which the global inputs section begins.
pub const GLOBAL_INPUTS_SECTION_OFFSET: MemoryOffset = 400;

/// The memory address at which the latest known block hash is stored.
pub const BLK_HASH_PTR: MemoryAddress = 400;

/// The memory address at which the account ID is stored.
pub const ACCT_ID_PTR: MemoryAddress = 404;

/// The memory address at which the initial account hash is stored.
pub const INIT_ACCT_HASH_PTR: MemoryAddress = 408;

/// The memory address at which the input notes commitment is stored.
pub const INPUT_NOTES_COMMITMENT_PTR: MemoryAddress = 412;

/// The memory address at which the initial nonce is stored.
pub const INIT_NONCE_PTR: MemoryAddress = 416;

/// The memory address at which the transaction script mast root is store
pub const TX_SCRIPT_ROOT_PTR: MemoryAddress = 420;

// BLOCK DATA
// ------------------------------------------------------------------------------------------------

/// The memory address at which the block data section begins
pub const BLOCK_DATA_SECTION_OFFSET: MemoryOffset = 800;

/// The memory address at which the previous block hash is stored
pub const PREV_BLOCK_HASH_PTR: MemoryAddress = 800;

/// The memory address at which the chain root is stored
pub const CHAIN_ROOT_PTR: MemoryAddress = 804;

/// The memory address at which the state root is stored
pub const ACCT_DB_ROOT_PTR: MemoryAddress = 808;

/// The memory address at which the nullifier db root is store
pub const NULLIFIER_DB_ROOT_PTR: MemoryAddress = 812;

/// The memory address at which the TX hash is stored
pub const TX_HASH_PTR: MemoryAddress = 816;

/// The memory address at which the kernel root is stored
pub const KERNEL_ROOT_PTR: MemoryAddress = 820;

/// The memory address at which the proof hash is stored
pub const PROOF_HASH_PTR: MemoryAddress = 824;

/// The memory address at which the block number is stored
pub const BLOCK_METADATA_PTR: MemoryAddress = 828;

/// The index of the block number within the block metadata
pub const BLOCK_NUMBER_IDX: DataIndex = 0;

/// The index of the protocol version within the block metadata
pub const PROTOCOL_VERSION_IDX: DataIndex = 1;

/// The index of the timestamp within the block metadata
pub const TIMESTAMP_IDX: DataIndex = 2;

/// The memory address at which the note root is stored
pub const NOTE_ROOT_PTR: MemoryAddress = 832;

// CHAIN DATA
// ------------------------------------------------------------------------------------------------

/// The memory address at which the chain data section begins
pub const CHAIN_MMR_PTR: MemoryAddress = 1200;

/// The memory address at which the total number of leaves in the chain MMR is stored
pub const CHAIN_MMR_NUM_LEAVES_PTR: MemoryAddress = 1200;

/// The memory address at which the chain mmr peaks are stored
pub const CHAIN_MMR_PEAKS_PTR: MemoryAddress = 1204;

// KERNEL DATA
// ------------------------------------------------------------------------------------------------

/// The memory address at which the number of the procedures of the selected kernel is stored.
pub const NUM_KERNEL_PROCEDURES_PTR: MemoryAddress = 1600;

/// The memory address at which the section, where the hashes of the kernel procedures are stored,
/// begins
pub const KERNEL_PROCEDURES_PTR: MemoryAddress = 1604;

// ACCOUNT DATA
// ------------------------------------------------------------------------------------------------

/// The size of the memory segment allocated to core account data (excluding new code commitment)
pub const ACCT_DATA_MEM_SIZE: MemSize = 16;

/// The memory address at which the native account is stored.
pub const NATIVE_ACCOUNT_DATA_PTR: MemoryAddress = 8192;

/// The length of the memory interval that the account data occupies.
pub const ACCOUNT_DATA_LENGTH: MemSize = 8192;

/// The offset at which the account ID and nonce are stored relative to the start of
/// the account data segment.
pub const ACCT_ID_AND_NONCE_OFFSET: MemoryOffset = 0;

/// The memory address at which the account ID and nonce are stored in the native account.
pub const NATIVE_ACCT_ID_AND_NONCE_PTR: MemoryAddress =
    NATIVE_ACCOUNT_DATA_PTR + ACCT_ID_AND_NONCE_OFFSET;

/// The index of the account ID within the account ID and nonce data.
pub const ACCT_ID_SUFFIX_IDX: DataIndex = 0;
pub const ACCT_ID_PREFIX_IDX: DataIndex = 1;

/// The index of the account nonce within the account ID and nonce data.
pub const ACCT_NONCE_IDX: DataIndex = 3;

/// The offset at which the account vault root is stored relative to the start of the account
/// data segment.
pub const ACCT_VAULT_ROOT_OFFSET: MemoryOffset = 4;

/// The memory address at which the account vault root is stored in the native account.
pub const NATIVE_ACCT_VAULT_ROOT_PTR: MemoryAddress =
    NATIVE_ACCOUNT_DATA_PTR + ACCT_VAULT_ROOT_OFFSET;

/// The offset at which the account storage commitment is stored relative to the start of the
/// account data segment.
pub const ACCT_STORAGE_COMMITMENT_OFFSET: MemoryOffset = 8;

/// The memory address at which the account storage commitment is stored in the native account.
pub const NATIVE_ACCT_STORAGE_COMMITMENT_PTR: MemoryAddress =
    NATIVE_ACCOUNT_DATA_PTR + ACCT_STORAGE_COMMITMENT_OFFSET;

/// The offset at which the account code commitment is stored relative to the start of the account
/// data segment.
pub const ACCT_CODE_COMMITMENT_OFFSET: MemoryOffset = 12;

/// The memory address at which the account code commitment is stored in the native account.
pub const NATIVE_ACCT_CODE_COMMITMENT_PTR: MemoryAddress =
    NATIVE_ACCOUNT_DATA_PTR + ACCT_CODE_COMMITMENT_OFFSET;

/// The offset at which the number of procedures contained in the account code is stored relative to
/// the start of the account data segment.
pub const NUM_ACCT_PROCEDURES_OFFSET: MemoryAddress = 28;

/// The memory address at which the number of procedures contained in the account code is stored in
/// the native account.
pub const NATIVE_NUM_ACCT_PROCEDURES_PTR: MemoryAddress =
    NATIVE_ACCOUNT_DATA_PTR + NUM_ACCT_PROCEDURES_OFFSET;

/// The offset at which the account procedures section begins relative to the start of the account
/// data segment.
pub const ACCT_PROCEDURES_SECTION_OFFSET: MemoryAddress = 32;

/// The memory address at which the account procedures section begins in the native account.
pub const NATIVE_ACCT_PROCEDURES_SECTION_PTR: MemoryAddress =
    NATIVE_ACCOUNT_DATA_PTR + ACCT_PROCEDURES_SECTION_OFFSET;

/// The offset at which the number of storage slots contained in the account storage is stored
/// relative to the start of the account data segment.
pub const NUM_ACCT_STORAGE_SLOTS_OFFSET: MemoryAddress = 2084;

/// The memory address at which number of storage slots contained in the account storage is stored
/// in the native account.
pub const NATIVE_NUM_ACCT_STORAGE_SLOTS_PTR: MemoryAddress =
    NATIVE_ACCOUNT_DATA_PTR + NUM_ACCT_STORAGE_SLOTS_OFFSET;

/// The offset at which the account storage slots section begins relative to the start of the
/// account data segment.
pub const ACCT_STORAGE_SLOTS_SECTION_OFFSET: MemoryAddress = 2088;

/// The memory address at which the account storage slots section begins in the native account.
pub const NATIVE_ACCT_STORAGE_SLOTS_SECTION_PTR: MemoryAddress =
    NATIVE_ACCOUNT_DATA_PTR + ACCT_STORAGE_SLOTS_SECTION_OFFSET;

// NOTES DATA
// ================================================================================================

/// The size of the memory segment allocated to each note.
pub const NOTE_MEM_SIZE: MemoryAddress = 2048;

#[rustfmt::skip]
// INPUT NOTES DATA
// ------------------------------------------------------------------------------------------------
// Inputs note section contains data of all notes consumed by a transaction. The section starts at
// memory offset 4_194_304 with a word containing the total number of input notes and is followed
// by note nullifiers and note data like so:
//
// ┌─────────┬───────────┬───────────┬─────┬───────────┬─────────┬────────┬────────┬───────┬────────┐
// │   NUM   │  NOTE 0   │  NOTE 1   │ ... │  NOTE n   │ PADDING │ NOTE 0 │ NOTE 1 │  ...  │ NOTE n │
// │  NOTES  │ NULLIFIER │ NULLIFIER │     │ NULLIFIER │         │  DATA  │  DATA  │       │  DATA  │
// └─────────┴───────────┴───────────┴─────┴───────────┴─────────┴────────┴────────┴───────┴────────┘
//  4_194_304  4_194_308   4_194_312        4_194_304+4n     4_259_840  +2048    +4096   +2048n
//
// Each nullifier occupies a single word. A data section for each note consists of exactly 512
// words and is laid out like so:
//
// ┌──────┬────────┬────────┬────────┬────────┬──────┬───────┬────────┬───────┬─────┬───────┬─────────┬
// │ NOTE │ SERIAL │ SCRIPT │ INPUTS │ ASSETS │ META │ NOTE  │   NUM  │ ASSET │ ... │ ASSET │ PADDING │
// │  ID  │  NUM   │  ROOT  │  HASH  │  HASH  │ DATA │ ARGS  │ ASSETS │   0   │     │   n   │         │
// ├──────┼────────┼────────┼────────┼────────┼──────┼───────┼────────┼───────┼─────┼───────┼─────────┤
//    0        4       8        12       16      20     24       28    32 + 4n
//
// - NUM_ASSETS is encoded [num_assets, 0, 0, 0].
// - INPUTS_HASH is the key to look up note inputs in the advice map.
// - ASSETS_HASH is the key to look up note assets in the advice map.

/// The memory address at which the input note section begins.
pub const INPUT_NOTE_SECTION_OFFSET: MemoryOffset = 4_194_304;

/// The memory address at which the input note data section begins.
pub const INPUT_NOTE_DATA_SECTION_OFFSET: MemoryAddress = 4_259_840;

/// The memory address at which the number of input notes is stored.
pub const NUM_INPUT_NOTES_PTR: MemoryAddress = INPUT_NOTE_SECTION_OFFSET;

/// The offsets at which data of an input note is stored relative to the start of its data segment.
pub const INPUT_NOTE_ID_OFFSET: MemoryOffset = 0;
pub const INPUT_NOTE_SERIAL_NUM_OFFSET: MemoryOffset = 4;
pub const INPUT_NOTE_SCRIPT_ROOT_OFFSET: MemoryOffset = 8;
pub const INPUT_NOTE_INPUTS_HASH_OFFSET: MemoryOffset = 12;
pub const INPUT_NOTE_ASSETS_HASH_OFFSET: MemoryOffset = 16;
pub const INPUT_NOTE_METADATA_OFFSET: MemoryOffset = 20;
pub const INPUT_NOTE_ARGS_OFFSET: MemoryOffset = 24;
pub const INPUT_NOTE_NUM_ASSETS_OFFSET: MemoryOffset = 28;
pub const INPUT_NOTE_ASSETS_OFFSET: MemoryOffset = 32;

// OUTPUT NOTES DATA
// ------------------------------------------------------------------------------------------------
// Output notes section contains data of all notes produced by a transaction. The section starts at
// memory offset 16_777_216 with each note data laid out one after another in 512 word increments.
//
//     ┌─────────────┬─────────────┬───────────────┬─────────────┐
//     │ NOTE 0 DATA │ NOTE 1 DATA │      ...      │ NOTE n DATA │
//     └─────────────┴─────────────┴───────────────┴─────────────┘
// 16_777_216      +2048         +4096           +2048n
//
// The total number of output notes for a transaction is stored in the bookkeeping section of the
// memory. Data section of each note is laid out like so:
//
// ┌─────────┬──────────┬───────────┬─────────────┬────────────┬─────────┬─────┬─────────┬─────────┐
// │ NOTE ID │ METADATA │ RECIPIENT │ ASSETS HASH │ NUM ASSETS │ ASSET 0 │ ... │ ASSET n │ PADDING │
// ├─────────┼──────────┼───────────┼─────────────┼────────────┼─────────┼─────┼─────────┼─────────┤
//      0          1          2            3            4           5             5 + n
//
// Even though NUM_ASSETS takes up a whole word, the actual value of this variable is stored in the
// first element of the word.

/// The memory address at which the output notes section begins.
pub const OUTPUT_NOTE_SECTION_OFFSET: MemoryOffset = 16_777_216;

/// The size of the core output note data segment.
pub const OUTPUT_NOTE_CORE_DATA_SIZE: MemSize = 16;

/// The offsets at which data of an output note is stored relative to the start of its data segment.
pub const OUTPUT_NOTE_ID_OFFSET: MemoryOffset = 0;
pub const OUTPUT_NOTE_METADATA_OFFSET: MemoryOffset = 4;
pub const OUTPUT_NOTE_RECIPIENT_OFFSET: MemoryOffset = 8;
pub const OUTPUT_NOTE_ASSET_HASH_OFFSET: MemoryOffset = 12;
pub const OUTPUT_NOTE_NUM_ASSETS_OFFSET: MemoryOffset = 16;
pub const OUTPUT_NOTE_ASSETS_OFFSET: MemoryOffset = 20;
