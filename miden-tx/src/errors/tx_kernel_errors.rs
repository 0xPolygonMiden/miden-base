// This file is generated by build.rs, do not modify manually.
// It is generated by extracting errors from the masm files in the `miden-lib/asm` directory.
//
// To add a new error, define a constant in masm of the pattern `const.ERR_<CATEGORY>_...`.
// Try to fit the error into a pre-existing category if possible (e.g. Account, Prologue, 
// Non-Fungible-Asset, ...).
//
// The comment directly above the constant will be interpreted as the error message for that error.

// KERNEL ASSERTION ERROR
// ================================================================================================

pub const ERR_ACCOUNT_CODE_COMMITMENT_MISMATCH: u32 = 0x0002000F;
pub const ERR_ACCOUNT_CODE_IS_NOT_UPDATABLE: u32 = 0x00020006;
pub const ERR_ACCOUNT_INSUFFICIENT_NUMBER_OF_ONES: u32 = 0x00020005;
pub const ERR_ACCOUNT_INVALID_STORAGE_OFFSET_FOR_SIZE: u32 = 0x00020013;
pub const ERR_ACCOUNT_IS_NOT_NATIVE: u32 = 0x00020030;
pub const ERR_ACCOUNT_NONCE_DID_NOT_INCREASE_AFTER_STATE_CHANGE: u32 = 0x00020028;
pub const ERR_ACCOUNT_NONCE_INCREASE_MUST_BE_U32: u32 = 0x00020004;
pub const ERR_ACCOUNT_POW_IS_INSUFFICIENT: u32 = 0x00020008;
pub const ERR_ACCOUNT_PROC_INDEX_OUT_OF_BOUNDS: u32 = 0x0002000C;
pub const ERR_ACCOUNT_PROC_NOT_PART_OF_ACCOUNT_CODE: u32 = 0x0002000B;
pub const ERR_ACCOUNT_READING_MAP_VALUE_FROM_NON_MAP_SLOT: u32 = 0x00020002;
pub const ERR_ACCOUNT_SEED_DIGEST_MISMATCH: u32 = 0x00020007;
pub const ERR_ACCOUNT_SETTING_MAP_ITEM_ON_NON_MAP_SLOT: u32 = 0x0002000A;
pub const ERR_ACCOUNT_SETTING_VALUE_ITEM_ON_NON_VALUE_SLOT: u32 = 0x00020009;
pub const ERR_ACCOUNT_STORAGE_COMMITMENT_MISMATCH: u32 = 0x00020012;
pub const ERR_ACCOUNT_TOO_MANY_PROCEDURES: u32 = 0x00020010;
pub const ERR_ACCOUNT_TOO_MANY_STORAGE_SLOTS: u32 = 0x00020011;
pub const ERR_ACCOUNT_TOTAL_ISSUANCE_PROC_CAN_ONLY_BE_CALLED_ON_FUNGIBLE_FAUCET: u32 = 0x00020001;

pub const ERR_EPILOGUE_TOTAL_NUMBER_OF_ASSETS_MUST_STAY_THE_SAME: u32 = 0x00020029;

pub const ERR_FAUCET_BURN_CANNOT_EXCEED_EXISTING_TOTAL_SUPPLY: u32 = 0x0002002B;
pub const ERR_FAUCET_BURN_NON_FUNGIBLE_ASSET_CAN_ONLY_BE_CALLED_ON_NON_FUNGIBLE_FAUCET: u32 = 0x0002002D;
pub const ERR_FAUCET_INVALID_STORAGE_OFFSET: u32 = 0x0002000E;
pub const ERR_FAUCET_NEW_TOTAL_SUPPLY_WOULD_EXCEED_MAX_ASSET_AMOUNT: u32 = 0x0002002A;
pub const ERR_FAUCET_NON_FUNGIBLE_ASSET_ALREADY_ISSUED: u32 = 0x0002002C;
pub const ERR_FAUCET_NON_FUNGIBLE_ASSET_TO_BURN_NOT_FOUND: u32 = 0x0002002E;
pub const ERR_FAUCET_STORAGE_DATA_SLOT_IS_RESERVED: u32 = 0x00020000;

pub const ERR_FOREIGN_ACCOUNT_ID_EQUALS_NATIVE_ACCT_ID: u32 = 0x00020016;
pub const ERR_FOREIGN_ACCOUNT_ID_IS_ZERO: u32 = 0x00020014;
pub const ERR_FOREIGN_ACCOUNT_INVALID: u32 = 0x00020017;
pub const ERR_FOREIGN_ACCOUNT_MAX_NUMBER_EXCEEDED: u32 = 0x00020015;

pub const ERR_FUNGIBLE_ASSET_AMOUNT_EXCEEDS_MAX_ALLOWED_AMOUNT: u32 = 0x0002004C;
pub const ERR_FUNGIBLE_ASSET_DISTRIBUTE_WOULD_CAUSE_MAX_SUPPLY_TO_BE_EXCEEDED: u32 = 0x0002004A;
pub const ERR_FUNGIBLE_ASSET_FAUCET_IS_NOT_ORIGIN: u32 = 0x00020026;
pub const ERR_FUNGIBLE_ASSET_FORMAT_ELEMENT_ONE_MUST_BE_ZERO: u32 = 0x00020020;
pub const ERR_FUNGIBLE_ASSET_FORMAT_ELEMENT_THREE_MUST_BE_FUNGIBLE_FAUCET_ID: u32 = 0x00020022;
pub const ERR_FUNGIBLE_ASSET_FORMAT_ELEMENT_TWO_MUST_BE_ZERO: u32 = 0x00020021;
pub const ERR_FUNGIBLE_ASSET_FORMAT_ELEMENT_ZERO_MUST_BE_WITHIN_LIMITS: u32 = 0x00020023;
pub const ERR_FUNGIBLE_ASSET_PROVIDED_FAUCET_ID_IS_INVALID: u32 = 0x0002004B;

pub const ERR_KERNEL_PROCEDURE_OFFSET_OUT_OF_BOUNDS: u32 = 0x00020003;

pub const ERR_NON_FUNGIBLE_ASSET_ALREADY_EXISTS: u32 = 0x00020047;
pub const ERR_NON_FUNGIBLE_ASSET_FAUCET_IS_NOT_ORIGIN: u32 = 0x00020027;
pub const ERR_NON_FUNGIBLE_ASSET_FORMAT_ELEMENT_ONE_MUST_BE_FUNGIBLE_FAUCET_ID: u32 = 0x00020024;
pub const ERR_NON_FUNGIBLE_ASSET_FORMAT_MOST_SIGNIFICANT_BIT_MUST_BE_ZERO: u32 = 0x00020025;
pub const ERR_NON_FUNGIBLE_ASSET_PROVIDED_FAUCET_ID_IS_INVALID: u32 = 0x0002004D;

pub const ERR_NOTE_ATTEMPT_TO_ACCESS_NOTE_ASSETS_FROM_INCORRECT_CONTEXT: u32 = 0x00020032;
pub const ERR_NOTE_ATTEMPT_TO_ACCESS_NOTE_INPUTS_FROM_INCORRECT_CONTEXT: u32 = 0x00020033;
pub const ERR_NOTE_ATTEMPT_TO_ACCESS_NOTE_SENDER_FROM_INCORRECT_CONTEXT: u32 = 0x00020031;
pub const ERR_NOTE_DATA_DOES_NOT_MATCH_COMMITMENT: u32 = 0x0002004E;
pub const ERR_NOTE_FUNGIBLE_MAX_AMOUNT_EXCEEDED: u32 = 0x00020046;
pub const ERR_NOTE_INVALID_INDEX: u32 = 0x00020048;
pub const ERR_NOTE_INVALID_NOTE_TYPE_FOR_NOTE_TAG_PREFIX: u32 = 0x00020044;
pub const ERR_NOTE_INVALID_TYPE: u32 = 0x00020043;
pub const ERR_NOTE_NUM_OF_ASSETS_EXCEED_LIMIT: u32 = 0x0002002F;
pub const ERR_NOTE_TAG_MUST_BE_U32: u32 = 0x00020045;

pub const ERR_P2IDR_RECLAIM_ACCT_IS_NOT_SENDER: u32 = 0x00020053;
pub const ERR_P2IDR_RECLAIM_HEIGHT_NOT_REACHED: u32 = 0x00020054;
pub const ERR_P2IDR_WRONG_NUMBER_OF_INPUTS: u32 = 0x00020052;

pub const ERR_P2ID_TARGET_ACCT_MISMATCH: u32 = 0x00020051;
pub const ERR_P2ID_WRONG_NUMBER_OF_INPUTS: u32 = 0x00020050;

pub const ERR_PROLOGUE_EXISTING_ACCOUNT_MUST_HAVE_NON_ZERO_NONCE: u32 = 0x0002003B;
pub const ERR_PROLOGUE_GLOBAL_INPUTS_PROVIDED_DO_NOT_MATCH_BLOCK_HASH_COMMITMENT: u32 = 0x00020034;
pub const ERR_PROLOGUE_INPUT_NOTES_COMMITMENT_MISMATCH: u32 = 0x00020041;
pub const ERR_PROLOGUE_MISMATCH_OF_ACCOUNT_IDS_FROM_GLOBAL_INPUTS_AND_ADVICE_PROVIDER: u32 = 0x0002003C;
pub const ERR_PROLOGUE_MISMATCH_OF_REFERENCE_BLOCK_MMR_AND_NOTE_AUTHENTICATION_MMR: u32 = 0x0002003D;
pub const ERR_PROLOGUE_NEW_ACCOUNT_VAULT_MUST_BE_EMPTY: u32 = 0x00020035;
pub const ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_RESERVED_SLOT_INVALID_TYPE: u32 = 0x00020037;
pub const ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_RESERVED_SLOT_MUST_BE_EMPTY: u32 = 0x00020036;
pub const ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_RESERVED_SLOT_INVALID_TYPE: u32 = 0x00020039;
pub const ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_RESERVED_SLOT_MUST_BE_VALID_EMPY_SMT: u32 = 0x00020038;
pub const ERR_PROLOGUE_NUMBER_OF_INPUT_NOTES_EXCEEDS_LIMIT: u32 = 0x00020040;
pub const ERR_PROLOGUE_NUMBER_OF_NOTE_ASSETS_EXCEEDS_LIMIT: u32 = 0x0002003E;
pub const ERR_PROLOGUE_NUMBER_OF_NOTE_INPUTS_EXCEEDED_LIMIT: u32 = 0x0002004F;
pub const ERR_PROLOGUE_PROVIDED_ACCOUNT_DATA_DOES_NOT_MATCH_ON_CHAIN_COMMITMENT: u32 = 0x0002003A;
pub const ERR_PROLOGUE_PROVIDED_INPUT_ASSETS_INFO_DOES_NOT_MATCH_ITS_COMMITMENT: u32 = 0x0002003F;

pub const ERR_STORAGE_SLOT_INDEX_OUT_OF_BOUNDS: u32 = 0x0002000D;

pub const ERR_SWAP_WRONG_NUMBER_OF_ASSETS: u32 = 0x00020056;
pub const ERR_SWAP_WRONG_NUMBER_OF_INPUTS: u32 = 0x00020055;

pub const ERR_TX_INVALID_EXPIRATION_DELTA: u32 = 0x00020049;
pub const ERR_TX_NUMBER_OF_OUTPUT_NOTES_EXCEEDS_LIMIT: u32 = 0x00020042;

pub const ERR_VAULT_ADD_FUNGIBLE_ASSET_FAILED_INITIAL_VALUE_INVALID: u32 = 0x0002001B;
pub const ERR_VAULT_FUNGIBLE_ASSET_AMOUNT_LESS_THAN_AMOUNT_TO_WITHDRAW: u32 = 0x0002001D;
pub const ERR_VAULT_FUNGIBLE_MAX_AMOUNT_EXCEEDED: u32 = 0x0002001A;
pub const ERR_VAULT_GET_BALANCE_PROC_CAN_ONLY_BE_CALLED_ON_FUNGIBLE_FAUCET: u32 = 0x00020018;
pub const ERR_VAULT_HAS_NON_FUNGIBLE_ASSET_PROC_CAN_BE_CALLED_ONLY_WITH_NON_FUNGIBLE_ASSET: u32 = 0x00020019;
pub const ERR_VAULT_NON_FUNGIBLE_ASSET_ALREADY_EXISTS: u32 = 0x0002001C;
pub const ERR_VAULT_NON_FUNGIBLE_ASSET_TO_REMOVE_NOT_FOUND: u32 = 0x0002001F;
pub const ERR_VAULT_REMOVE_FUNGIBLE_ASSET_FAILED_INITIAL_VALUE_INVALID: u32 = 0x0002001E;

pub const TX_KERNEL_ERRORS: [(u32, &str); 87] = [
    (ERR_ACCOUNT_CODE_COMMITMENT_MISMATCH, "Computed account code commitment does not match recorded account code commitment"),
    (ERR_ACCOUNT_CODE_IS_NOT_UPDATABLE, "Account code must be updatable for it to be possible to set new code"),
    (ERR_ACCOUNT_INSUFFICIENT_NUMBER_OF_ONES, "Account ID must contain at least MIN_ACCOUNT_ONES number of ones"),
    (ERR_ACCOUNT_INVALID_STORAGE_OFFSET_FOR_SIZE, "Storage offset is invalid for 0 storage size (should be 0)"),
    (ERR_ACCOUNT_IS_NOT_NATIVE, "The current account is not native"),
    (ERR_ACCOUNT_NONCE_DID_NOT_INCREASE_AFTER_STATE_CHANGE, "Account nonce did not increase after a state changing transaction"),
    (ERR_ACCOUNT_NONCE_INCREASE_MUST_BE_U32, "Account nonce cannot be increased by a greater than u32 value"),
    (ERR_ACCOUNT_POW_IS_INSUFFICIENT, "Account proof of work is insufficient"),
    (ERR_ACCOUNT_PROC_INDEX_OUT_OF_BOUNDS, "Provided procedure index is out of bounds"),
    (ERR_ACCOUNT_PROC_NOT_PART_OF_ACCOUNT_CODE, "Account procedure is not part of the account code"),
    (ERR_ACCOUNT_READING_MAP_VALUE_FROM_NON_MAP_SLOT, "Failed to read an account map item from a non-map storage slot"),
    (ERR_ACCOUNT_SEED_DIGEST_MISMATCH, "ID of the new account does not match the ID computed from the seed"),
    (ERR_ACCOUNT_SETTING_MAP_ITEM_ON_NON_MAP_SLOT, "Failed to write an account map item to a non-map storage slot"),
    (ERR_ACCOUNT_SETTING_VALUE_ITEM_ON_NON_VALUE_SLOT, "Failed to write an account value item to a non-value storage slot"),
    (ERR_ACCOUNT_STORAGE_COMMITMENT_MISMATCH, "Computed account storage commitment does not match recorded account storage commitment"),
    (ERR_ACCOUNT_TOO_MANY_PROCEDURES, "Number of account procedures exceeds the maximum limit of 256"),
    (ERR_ACCOUNT_TOO_MANY_STORAGE_SLOTS, "Number of account storage slots exceeds the maximum limit of 255"),
    (ERR_ACCOUNT_TOTAL_ISSUANCE_PROC_CAN_ONLY_BE_CALLED_ON_FUNGIBLE_FAUCET, "The get_fungible_faucet_total_issuance procedure can only be called on a fungible faucet"),

    (ERR_EPILOGUE_TOTAL_NUMBER_OF_ASSETS_MUST_STAY_THE_SAME, "Total number of assets in the account and all involved notes must stay the same"),

    (ERR_FAUCET_BURN_CANNOT_EXCEED_EXISTING_TOTAL_SUPPLY, "Asset amount to burn can not exceed the existing total supply"),
    (ERR_FAUCET_BURN_NON_FUNGIBLE_ASSET_CAN_ONLY_BE_CALLED_ON_NON_FUNGIBLE_FAUCET, "The burn_non_fungible_asset procedure can only be called on a non-fungible faucet"),
    (ERR_FAUCET_INVALID_STORAGE_OFFSET, "Storage offset is invalid for a faucet account (0 is prohibited as it is the reserved data slot for faucets)"),
    (ERR_FAUCET_NEW_TOTAL_SUPPLY_WOULD_EXCEED_MAX_ASSET_AMOUNT, "Asset mint operation would cause the new total supply to exceed the maximum allowed asset amount"),
    (ERR_FAUCET_NON_FUNGIBLE_ASSET_ALREADY_ISSUED, "Failed to mint new non-fungible asset because it was already issued"),
    (ERR_FAUCET_NON_FUNGIBLE_ASSET_TO_BURN_NOT_FOUND, "Failed to burn non-existent non-fungible asset in the vault"),
    (ERR_FAUCET_STORAGE_DATA_SLOT_IS_RESERVED, "For faucets the FAUCET_STORAGE_DATA_SLOT storage slot is reserved and can not be used with set_account_item"),

    (ERR_FOREIGN_ACCOUNT_ID_EQUALS_NATIVE_ACCT_ID, "Provided foreign account ID is equal to the native account ID."),
    (ERR_FOREIGN_ACCOUNT_ID_IS_ZERO, "ID of the provided foreign account equals zero."),
    (ERR_FOREIGN_ACCOUNT_INVALID, "State of the current foreign account is invalid."),
    (ERR_FOREIGN_ACCOUNT_MAX_NUMBER_EXCEEDED, "Maximum allowed number of foreign account to be loaded (64) was exceeded."),

    (ERR_FUNGIBLE_ASSET_AMOUNT_EXCEEDS_MAX_ALLOWED_AMOUNT, "Fungible asset build operation called with amount that exceeds the maximum allowed asset amount"),
    (ERR_FUNGIBLE_ASSET_DISTRIBUTE_WOULD_CAUSE_MAX_SUPPLY_TO_BE_EXCEEDED, "Distribute would cause the maximum supply to be exceeded"),
    (ERR_FUNGIBLE_ASSET_FAUCET_IS_NOT_ORIGIN, "The origin of the fungible asset is not this faucet"),
    (ERR_FUNGIBLE_ASSET_FORMAT_ELEMENT_ONE_MUST_BE_ZERO, "Malformed fungible asset: ASSET[1] must be 0"),
    (ERR_FUNGIBLE_ASSET_FORMAT_ELEMENT_THREE_MUST_BE_FUNGIBLE_FAUCET_ID, "Malformed fungible asset: ASSET[3] must be a valide fungible faucet id"),
    (ERR_FUNGIBLE_ASSET_FORMAT_ELEMENT_TWO_MUST_BE_ZERO, "Malformed fungible asset: ASSET[2] must be 0"),
    (ERR_FUNGIBLE_ASSET_FORMAT_ELEMENT_ZERO_MUST_BE_WITHIN_LIMITS, "Malformed fungible asset: ASSET[0] exceeds the maximum allowed amount"),
    (ERR_FUNGIBLE_ASSET_PROVIDED_FAUCET_ID_IS_INVALID, "Failed to build the fungible asset because the provided faucet id is not from a fungible faucet"),

    (ERR_KERNEL_PROCEDURE_OFFSET_OUT_OF_BOUNDS, "Provided kernel procedure offset is out of bounds"),

    (ERR_NON_FUNGIBLE_ASSET_ALREADY_EXISTS, "Non-fungible asset that already exists in the note cannot be added again"),
    (ERR_NON_FUNGIBLE_ASSET_FAUCET_IS_NOT_ORIGIN, "The origin of the non-fungible asset is not this faucet"),
    (ERR_NON_FUNGIBLE_ASSET_FORMAT_ELEMENT_ONE_MUST_BE_FUNGIBLE_FAUCET_ID, "Malformed non-fungible asset: ASSET[1] is not a valid non-fungible faucet id"),
    (ERR_NON_FUNGIBLE_ASSET_FORMAT_MOST_SIGNIFICANT_BIT_MUST_BE_ZERO, "Malformed non-fungible asset: the most significant bit must be 0"),
    (ERR_NON_FUNGIBLE_ASSET_PROVIDED_FAUCET_ID_IS_INVALID, "Failed to build the non-fungible asset because the provided faucet id is not from a non-fungible faucet"),

    (ERR_NOTE_ATTEMPT_TO_ACCESS_NOTE_ASSETS_FROM_INCORRECT_CONTEXT, "Attempted to access note assets from incorrect context"),
    (ERR_NOTE_ATTEMPT_TO_ACCESS_NOTE_INPUTS_FROM_INCORRECT_CONTEXT, "Attempted to access note inputs from incorrect context"),
    (ERR_NOTE_ATTEMPT_TO_ACCESS_NOTE_SENDER_FROM_INCORRECT_CONTEXT, "Attempted to access note sender from incorrect context"),
    (ERR_NOTE_DATA_DOES_NOT_MATCH_COMMITMENT, "Note data does not match the commitment"),
    (ERR_NOTE_FUNGIBLE_MAX_AMOUNT_EXCEEDED, "Adding a fungible asset to a note cannot exceed the max_amount of 9223372036854775807"),
    (ERR_NOTE_INVALID_INDEX, "Failed to find note at the given index; index must be within [0, num_of_notes]"),
    (ERR_NOTE_INVALID_NOTE_TYPE_FOR_NOTE_TAG_PREFIX, "Invalid note type for the given note tag prefix"),
    (ERR_NOTE_INVALID_TYPE, "Invalid note type"),
    (ERR_NOTE_NUM_OF_ASSETS_EXCEED_LIMIT, "Number of assets in a note exceed 255"),
    (ERR_NOTE_TAG_MUST_BE_U32, "The note's tag must fit into a u32 so the 32 most significant bits must be zero."),

    (ERR_P2IDR_RECLAIM_ACCT_IS_NOT_SENDER, "P2IDR's reclaimer is not the original sender"),
    (ERR_P2IDR_RECLAIM_HEIGHT_NOT_REACHED, "P2IDR can not be reclaimed as the transaction's reference block is lower than the reclaim height"),
    (ERR_P2IDR_WRONG_NUMBER_OF_INPUTS, "P2IDR scripts expect exactly 2 note inputs"),

    (ERR_P2ID_TARGET_ACCT_MISMATCH, "P2ID's target account address and transaction address do not match"),
    (ERR_P2ID_WRONG_NUMBER_OF_INPUTS, "P2ID script expects exactly 1 note input"),

    (ERR_PROLOGUE_EXISTING_ACCOUNT_MUST_HAVE_NON_ZERO_NONCE, "Existing accounts must have a non-zero nonce"),
    (ERR_PROLOGUE_GLOBAL_INPUTS_PROVIDED_DO_NOT_MATCH_BLOCK_HASH_COMMITMENT, "The provided global inputs do not match the block hash commitment"),
    (ERR_PROLOGUE_INPUT_NOTES_COMMITMENT_MISMATCH, "Note commitment computed from the input note data does not match given note commitment"),
    (ERR_PROLOGUE_MISMATCH_OF_ACCOUNT_IDS_FROM_GLOBAL_INPUTS_AND_ADVICE_PROVIDER, "Account IDs provided via global inputs and advice provider do not match"),
    (ERR_PROLOGUE_MISMATCH_OF_REFERENCE_BLOCK_MMR_AND_NOTE_AUTHENTICATION_MMR, "Reference block MMR and note's authentication MMR must match"),
    (ERR_PROLOGUE_NEW_ACCOUNT_VAULT_MUST_BE_EMPTY, "New account must have an empty vault"),
    (ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_RESERVED_SLOT_INVALID_TYPE, "Reserved slot for new fungible faucet has an invalid type"),
    (ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_RESERVED_SLOT_MUST_BE_EMPTY, "Reserved slot for new fungible faucet is not empty"),
    (ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_RESERVED_SLOT_INVALID_TYPE, "Reserved slot for new non-fungible faucet has an invalid type"),
    (ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_RESERVED_SLOT_MUST_BE_VALID_EMPY_SMT, "Reserved slot for non-fungible faucet is not a valid empty SMT"),
    (ERR_PROLOGUE_NUMBER_OF_INPUT_NOTES_EXCEEDS_LIMIT, "Number of input notes exceeds the kernel's maximum limit of 1024"),
    (ERR_PROLOGUE_NUMBER_OF_NOTE_ASSETS_EXCEEDS_LIMIT, "Number of note assets exceeds the maximum limit of 256"),
    (ERR_PROLOGUE_NUMBER_OF_NOTE_INPUTS_EXCEEDED_LIMIT, "Number of note inputs exceeded the maximum limit of 128"),
    (ERR_PROLOGUE_PROVIDED_ACCOUNT_DATA_DOES_NOT_MATCH_ON_CHAIN_COMMITMENT, "Account data provided does not match the commitment recorded on-chain"),
    (ERR_PROLOGUE_PROVIDED_INPUT_ASSETS_INFO_DOES_NOT_MATCH_ITS_COMMITMENT, "Provided info about assets of an input does not match its commitment"),

    (ERR_STORAGE_SLOT_INDEX_OUT_OF_BOUNDS, "Provided storage slot index is out of bounds"),

    (ERR_SWAP_WRONG_NUMBER_OF_ASSETS, "SWAP script requires exactly 1 note asset"),
    (ERR_SWAP_WRONG_NUMBER_OF_INPUTS, "SWAP script expects exactly 10 note inputs"),

    (ERR_TX_INVALID_EXPIRATION_DELTA, "Transaction expiration block delta must be within 0x1 and 0xFFFF."),
    (ERR_TX_NUMBER_OF_OUTPUT_NOTES_EXCEEDS_LIMIT, "Number of output notes in the transaction exceeds the maximum limit of 1024"),

    (ERR_VAULT_ADD_FUNGIBLE_ASSET_FAILED_INITIAL_VALUE_INVALID, "Failed to add fungible asset to the asset vault due to the initial value being invalid"),
    (ERR_VAULT_FUNGIBLE_ASSET_AMOUNT_LESS_THAN_AMOUNT_TO_WITHDRAW, "Failed to remove the fungible asset from the vault since the amount of the asset in the vault is less than the amount to remove"),
    (ERR_VAULT_FUNGIBLE_MAX_AMOUNT_EXCEEDED, "Adding the fungible asset to the vault would exceed the max amount of 9223372036854775807"),
    (ERR_VAULT_GET_BALANCE_PROC_CAN_ONLY_BE_CALLED_ON_FUNGIBLE_FAUCET, "The get_balance procedure can only be called on a fungible faucet"),
    (ERR_VAULT_HAS_NON_FUNGIBLE_ASSET_PROC_CAN_BE_CALLED_ONLY_WITH_NON_FUNGIBLE_ASSET, "The has_non_fungible_asset procedure can only be called on a non-fungible faucet"),
    (ERR_VAULT_NON_FUNGIBLE_ASSET_ALREADY_EXISTS, "The non-fungible asset already exists in the asset vault"),
    (ERR_VAULT_NON_FUNGIBLE_ASSET_TO_REMOVE_NOT_FOUND, "Failed to remove non-existent non-fungible asset from the vault"),
    (ERR_VAULT_REMOVE_FUNGIBLE_ASSET_FAILED_INITIAL_VALUE_INVALID, "Failed to remove fungible asset from the asset vault due to the initial value being invalid"),
];
