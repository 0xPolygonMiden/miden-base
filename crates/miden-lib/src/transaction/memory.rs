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
// | Section           | Start address, pointer (word pointer) | End address, pointer (word pointer) | Comment                                     |
// | ----------------- | :-----------------------------------: | :---------------------------------: | ------------------------------------------- |
// | Bookkeeping       | 0 (0)                                 | 287 (71)                            |                                             |
// | Global inputs     | 400 (100)                             | 423 (105)                           |                                             |
// | Block header      | 800 (200)                             | 835 (208)                           |                                             |
// | Chain MMR         | 1_200 (300)                           | 1_331? (332?)                       |                                             |
// | Kernel data       | 1_600 (400)                           | 1_739 (434)                         | 34 procedures in total, 4 elements each     |
// | Accounts data     | 8_192 (2048)                          | 532_479 (133_119)                   | 64 accounts max, 8192 elements each         |
// | Input notes       | 4_194_304 (1_048_576)                 | ?                                   |                                             |
// | Output notes      | 16_777_216 (4_194_304)                | ?                                   |                                             |

// Relative layout of one account
//
// Here the "end pointer" is the last memory pointer occupied by the current data
//
// | Section           | Start address, pointer (word pointer) | End address, pointer (word pointer) | Comment                             |
// | ----------------- | :-----------------------------------: | :---------------------------------: | ----------------------------------- |
// | Id and nonce      | 0 (0)                                 | 3 (0)                               |                                     |
// | Vault root        | 4 (1)                                 | 7 (1)                               |                                     |
// | Storage root      | 8 (2)                                 | 11 (2)                              |                                     |
// | Code root         | 12 (3)                                | 15 (3)                              |                                     |
// | Padding           | 16 (4)                                | 27 (6)                              |                                     |
// | Num procedures    | 28 (7)                                | 31 (7)                              |                                     |
// | Procedures info   | 32 (8)                                | 2_079 (519)                         | 255 procedures max, 8 elements each |
// | Padding           | 2_080 (520)                           | 2_083 (520)                         |                                     |
// | Num storage slots | 2_084 (521)                           | 2_087 (521)                         |                                     |
// | Storage slot info | 2_088 (522)                           | 4_127 (1031)                        | 255 slots max, 8 elements each      |
// | Padding           | 4_128 (1032)                          | 8_191 (2047)                        |                                     |

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

/// The memory address at which the input vault root is stored.
pub const INPUT_VAULT_ROOT_PTR: MemoryAddress = 12;

/// The memory address at which the output vault root is stored.
pub const OUTPUT_VAULT_ROOT_PTR: MemoryAddress = 16;

/// The memory address at which the native account's new code commitment is stored.
pub const NEW_CODE_ROOT_PTR: MemoryAddress = 20;

/// The memory address at which the transaction expiration block number is stored.
pub const TX_EXPIRATION_BLOCK_NUM_PTR: MemoryAddress = 24;

/// The memory address at which the pointer to the stack element containing the pointer to the
/// currently active account data is stored.
///
/// The stack starts at the address `29`. Stack has a length of `64` elements meaning that the
/// maximum depth of FPI calls is `63` — the first slot is always occupied by the native account
/// data pointer.
///
/// ```text
/// ┌───────────────┬────────────────┬───────────────────┬─────┬────────────────────┐
/// │ STACK TOP PTR │ NATIVE ACCOUNT │ FOREIGN ACCOUNT 1 │ ... │ FOREIGN ACCOUNT 63 │
/// ├───────────────┼────────────────┼───────────────────┼─────┼────────────────────┤
///        28               29                30                         92
/// ```
pub const ACCOUNT_STACK_TOP_PTR: MemoryAddress = 28;

// GLOBAL INPUTS
// ------------------------------------------------------------------------------------------------

/// The memory address at which the global inputs section begins.
pub const GLOBAL_INPUTS_SECTION_OFFSET: MemoryOffset = 400;

/// The memory address at which the commitment of the transaction's reference block is stored.
pub const BLOCK_COMMITMENT_PTR: MemoryAddress = 400;

/// The memory address at which the account ID is stored.
pub const ACCT_ID_PTR: MemoryAddress = 404;

/// The memory address at which the initial account commitment is stored.
pub const INIT_ACCT_COMMITMENT_PTR: MemoryAddress = 408;

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

/// The memory address at which the previous block commitment is stored
pub const PREV_BLOCK_COMMITMENT_PTR: MemoryAddress = 800;

/// The memory address at which the chain commitment is stored
pub const CHAIN_COMMITMENT_PTR: MemoryAddress = 804;

/// The memory address at which the state root is stored
pub const ACCT_DB_ROOT_PTR: MemoryAddress = 808;

/// The memory address at which the nullifier db root is store
pub const NULLIFIER_DB_ROOT_PTR: MemoryAddress = 812;

/// The memory address at which the TX commitment is stored
pub const TX_COMMITMENT_PTR: MemoryAddress = 816;

/// The memory address at which the transaction kernel commitment is stored
pub const TX_KERNEL_COMMITMENT_PTR: MemoryAddress = 820;

/// The memory address at which the proof commitment is stored
pub const PROOF_COMMITMENT_PTR: MemoryAddress = 824;

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

#[allow(clippy::empty_line_after_outer_attr)]
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
//  4_194_304  4_194_308  4_194_312    4_194_304+4(n+1)      4_259_840  +2048    +4096  +2048(n+1)
//
// Each nullifier occupies a single word. A data section for each note consists of exactly 512
// words and is laid out like so:
//
// ┌──────┬────────┬────────┬────────┬────────┬──────┬───────┬────────┬───────┬─────┬───────┬─────────┬
// │ NOTE │ SERIAL │ SCRIPT │ INPUTS │ ASSETS │ META │ NOTE  │   NUM  │ ASSET │ ... │ ASSET │ PADDING │
// │  ID  │  NUM   │  ROOT  │  HASH  │  HASH  │ DATA │ ARGS  │ ASSETS │   0   │     │   n   │         │
// ├──────┼────────┼────────┼────────┼────────┼──────┼───────┼────────┼───────┼─────┼───────┼─────────┤
// 0      4        8        12       16       20     24      28       32 + 4n
//
// - NUM_ASSETS is encoded [num_assets, 0, 0, 0].
// - INPUTS_COMMITMENT is the key to look up note inputs in the advice map.
// - ASSETS_HASH is the key to look up note assets in the advice map.

/// The memory address at which the input note section begins.
pub const INPUT_NOTE_SECTION_PTR: MemoryAddress = 4_194_304;

/// The memory address at which the nullifier section of the input notes begins.
pub const INPUT_NOTE_NULLIFIER_SECTION_PTR: MemoryAddress = 4_194_308;

/// The memory address at which the input note data section begins.
pub const INPUT_NOTE_DATA_SECTION_OFFSET: MemoryAddress = 4_259_840;

/// The memory address at which the number of input notes is stored.
pub const NUM_INPUT_NOTES_PTR: MemoryAddress = INPUT_NOTE_SECTION_PTR;

/// The offsets at which data of an input note is stored relative to the start of its data segment.
pub const INPUT_NOTE_ID_OFFSET: MemoryOffset = 0;
pub const INPUT_NOTE_SERIAL_NUM_OFFSET: MemoryOffset = 4;
pub const INPUT_NOTE_SCRIPT_ROOT_OFFSET: MemoryOffset = 8;
pub const INPUT_NOTE_INPUTS_COMMITMENT_OFFSET: MemoryOffset = 12;
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
pub const OUTPUT_NOTE_ASSET_COMMITMENT_OFFSET: MemoryOffset = 12;
pub const OUTPUT_NOTE_NUM_ASSETS_OFFSET: MemoryOffset = 16;
pub const OUTPUT_NOTE_ASSETS_OFFSET: MemoryOffset = 20;

// ASSETS

pub const ASSET_ID_SEED_PTR: u32 = 33554432;
pub const ASSET_NUM_CREATED: u32 = 33554436;
pub const ASSET_NEXT_OFFSET_PTR: u32 = 33554437;
pub const ASSET_PTR_MAP_MIN: u32 = 33554440;
pub const ASSET_PTR_MAP_MAX: u32 = 33554696;
pub const ASSET_MIN_PTR: u32 = 33554728;
pub const ASSET_BOOKKEEPING_SIZE: u32 = 8;
pub const ASSET_ISSUER_PREFIX_OFFSET: u32 = 4;
pub const ASSET_ISSUER_SUFFIX_OFFSET: u32 = 5;
pub const ASSET_TYPE_ID_OFFSET: u32 = 6;
pub const ASSET_NUM_FIELDS_OFFSET: u32 = 7;
