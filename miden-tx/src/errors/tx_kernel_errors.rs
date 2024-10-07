// KERNEL ASSERTION ERROR
// ================================================================================================

pub const ERR_FAUCET_RESERVED_DATA_SLOT: u32 = 131072;
pub const ERR_ACCT_MUST_BE_A_FAUCET: u32 = 131073;
pub const ERR_P2ID_WRONG_NUMBER_OF_INPUTS: u32 = 131074;
pub const ERR_P2ID_TARGET_ACCT_MISMATCH: u32 = 131075;
pub const ERR_P2IDR_WRONG_NUMBER_OF_INPUTS: u32 = 131076;
pub const ERR_P2IDR_RECLAIM_ACCT_IS_NOT_SENDER: u32 = 131077;
pub const ERR_P2IDR_RECLAIM_HEIGHT_NOT_REACHED: u32 = 131078;
pub const ERR_SWAP_WRONG_NUMBER_OF_INPUTS: u32 = 131079;
pub const ERR_SWAP_WRONG_NUMBER_OF_ASSETS: u32 = 131080;
pub const ERR_NONCE_DID_NOT_INCREASE: u32 = 131081;
pub const ERR_EPILOGUE_ASSETS_DONT_ADD_UP: u32 = 131082;
pub const ERR_PROLOGUE_GLOBAL_INPUTS_MISMATCH: u32 = 131083;
pub const ERR_PROLOGUE_ACCT_STORAGE_MISMATCH: u32 = 131084;
pub const ERR_PROLOGUE_ACCT_STORAGE_ARITY_TOO_HIGH: u32 = 131085;
pub const ERR_PROLOGUE_ACCT_STORAGE_MODE_INVALID: u32 = 131086;
pub const ERR_PROLOGUE_NEW_ACCT_VAULT_NOT_EMPTY: u32 = 131087;
pub const ERR_PROLOGUE_NEW_ACCT_INVALID_SLOT_TYPE: u32 = 131088;
pub const ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_NON_EMPTY_RESERVED_SLOT: u32 = 131089;
pub const ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_NON_ZERO_RESERVED_SLOT: u32 = 131090;
pub const ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_INVALID_TYPE_RESERVED_SLOT: u32 = 131091;
pub const ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_INVALID_RESERVED_SLOT: u32 = 131092;
pub const ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_NON_ZERO_RESERVED_SLOT: u32 = 131093;
pub const ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_INVALID_TYPE_RESERVED_SLOT: u32 = 131094;
pub const ERR_PROLOGUE_ACCT_HASH_MISMATCH: u32 = 131095;
pub const ERR_PROLOGUE_OLD_ACCT_NONCE_ZERO: u32 = 131096;
pub const ERR_PROLOGUE_ACCT_ID_MISMATCH: u32 = 131097;
pub const ERR_PROLOGUE_NOTE_MMR_DIGEST_MISMATCH: u32 = 131098;
pub const ERR_NOTE_TOO_MANY_INPUTS: u32 = 131099;
pub const ERR_PROLOGUE_NOTE_TOO_MANY_ASSETS: u32 = 131100;
pub const ERR_PROLOGUE_NOTE_CONSUMED_ASSETS_MISMATCH: u32 = 131101;
pub const ERR_PROLOGUE_TOO_MANY_INPUT_NOTES: u32 = 131102;
pub const ERR_PROLOGUE_INPUT_NOTES_COMMITMENT_MISMATCH: u32 = 131103;
pub const ERR_TX_OUTPUT_NOTES_OVERFLOW: u32 = 131104;
pub const ERR_BASIC_FUNGIBLE_MAX_SUPPLY_OVERFLOW: u32 = 131105;
pub const ERR_FAUCET_ISSUANCE_OVERFLOW: u32 = 131106;
pub const ERR_FAUCET_BURN_OVER_ISSUANCE: u32 = 131107;
pub const ERR_FAUCET_NON_FUNGIBLE_ALREADY_EXISTS: u32 = 131108;
pub const ERR_FAUCET_NON_FUNGIBLE_BURN_WRONG_TYPE: u32 = 131109;
pub const ERR_FAUCET_NONEXISTING_TOKEN: u32 = 131110;
pub const ERR_NOTE_INVALID_SENDER: u32 = 131111;
pub const ERR_NOTE_INVALID_ASSETS: u32 = 131112;
pub const ERR_NOTE_INVALID_INPUTS: u32 = 131113;
pub const ERR_NOTE_TOO_MANY_ASSETS: u32 = 131114;
pub const ERR_VAULT_GET_BALANCE_WRONG_ASSET_TYPE: u32 = 131115;
pub const ERR_VAULT_HAS_NON_FUNGIBLE_WRONG_ACCOUNT_TYPE: u32 = 131116;
pub const ERR_VAULT_FUNGIBLE_MAX_AMOUNT_EXCEEDED: u32 = 131117;
pub const ERR_VAULT_ADD_FUNGIBLE_ASSET_MISMATCH: u32 = 131118;
pub const ERR_VAULT_NON_FUNGIBLE_ALREADY_EXISTS: u32 = 131119;
pub const ERR_VAULT_FUNGIBLE_AMOUNT_UNDERFLOW: u32 = 131120;
pub const ERR_VAULT_REMOVE_FUNGIBLE_ASSET_MISMATCH: u32 = 131121;
pub const ERR_VAULT_NON_FUNGIBLE_MISSING_ASSET: u32 = 131122;
pub const ERR_FUNGIBLE_ASSET_FORMAT_POSITION_ONE_MUST_BE_ZERO: u32 = 131123;
pub const ERR_FUNGIBLE_ASSET_FORMAT_POSITION_TWO_MUST_BE_ZERO: u32 = 131124;
pub const ERR_FUNGIBLE_ASSET_FORMAT_POSITION_THREE_MUST_BE_FUNGIBLE_FAUCET_ID: u32 = 131125;
pub const ERR_FUNGIBLE_ASSET_FORMAT_POSITION_ZERO_MUST_BE_WITHIN_LIMITS: u32 = 131126;
pub const ERR_NON_FUNGIBLE_ASSET_FORMAT_POSITION_ONE_MUST_BE_FUNGIBLE_FAUCET_ID: u32 = 131127;
pub const ERR_NON_FUNGIBLE_ASSET_FORMAT_HIGH_BIT_MUST_BE_ZERO: u32 = 131128;
pub const ERR_FUNGIBLE_ASSET_MISMATCH: u32 = 131129;
pub const ERR_NON_FUNGIBLE_ASSET_MISMATCH: u32 = 131130;
pub const ERR_ACCOUNT_NONCE_INCR_MUST_BE_U32: u32 = 131131;
pub const ERR_ACCOUNT_INSUFFICIENT_ONES: u32 = 131132;
pub const ERR_ACCOUNT_SET_CODE_ACCOUNT_MUST_BE_UPDATABLE: u32 = 131133;
pub const ERR_ACCOUNT_SEED_DIGEST_MISMATCH: u32 = 131134;
pub const ERR_ACCOUNT_INVALID_POW: u32 = 131135;
pub const ERR_NOTE_DATA_MISMATCH: u32 = 131136;
pub const ERR_ASSET_NOT_FUNGIBLE_ID: u32 = 131137;
pub const ERR_ASSET_INVALID_AMOUNT: u32 = 131138;
pub const ERR_ASSET_NOT_NON_FUNGIBLE_ID: u32 = 131139;
pub const ERR_INVALID_NOTE_TYPE: u32 = 131140;
pub const ERR_NOTE_INVALID_TYPE_FOR_TAG: u32 = 131141;
pub const ERR_NOTE_TAG_MUST_BE_U32: u32 = 131142;
pub const ERR_SETTING_NON_VALUE_ITEM_ON_VALUE_SLOT: u32 = 131143;
pub const ERR_SETTING_MAP_ITEM_ON_NON_MAP_SLOT: u32 = 131144;
pub const ERR_READING_MAP_VALUE_FROM_NON_MAP_SLOT: u32 = 131145;
pub const ERR_PROC_NOT_PART_OF_ACCOUNT_CODE: u32 = 131146;
pub const ERR_PROC_INDEX_OUT_OF_BOUNDS: u32 = 131147;
pub const ERR_ACCT_CODE_HASH_MISMATCH: u32 = 131148;
pub const ERR_ACCT_TOO_MANY_PROCEDURES: u32 = 131149;
pub const ERR_STORAGE_SLOT_INDEX_OUT_OF_BOUNDS: u32 = 131150;
pub const ERR_INVALID_FAUCET_STORAGE_OFFSET: u32 = 131151;
pub const ERR_NOTE_FUNGIBLE_MAX_AMOUNT_EXCEEDED: u32 = 131152;
pub const ERR_NON_FUNGIBLE_ASSET_ALREADY_EXISTS: u32 = 131153;
pub const ERR_INVALID_NOTE_IDX: u32 = 131154;
pub const ERR_KERNEL_PROCEDURE_OFFSET_OUT_OF_BOUNDS: u32 = 131155;
pub const ERR_CURRENT_ACCOUNT_IS_NOT_NATIVE: u32 = 131156;
pub const ERR_INVALID_TX_EXPIRATION_DELTA: u32 = 131157;
pub const ERR_FOREIGN_ACCOUNT_ID_IS_ZERO: u32 = 131158;
pub const ERR_MAX_NUM_FOREIGN_ACCOUNTS_EXCEEDED: u32 = 131159;
pub const ERR_ACCT_TOO_MANY_STORAGE_SLOTS: u32 = 131160;
pub const ERR_FOREIGN_ACCT_ID_EQUALS_NATIVE_ACCT_ID: u32 = 131161;
pub const ERR_INVALID_STORAGE_OFFSET_FOR_SIZE: u32 = 131162;

pub const KERNEL_ERRORS: [(u32, &str); 91] = [
    (ERR_FAUCET_RESERVED_DATA_SLOT, "For faucets, storage slot 254 is reserved and can not be used with set_account_item procedure"),
    (ERR_ACCT_MUST_BE_A_FAUCET, "Procedure can only be called from faucet accounts"),
    (ERR_P2ID_WRONG_NUMBER_OF_INPUTS, "P2ID scripts expect exactly 1 note input"),
    (ERR_P2ID_TARGET_ACCT_MISMATCH, "P2ID's target account address and transaction address do not match"),
    (ERR_P2IDR_WRONG_NUMBER_OF_INPUTS, "P2IDR scripts expect exactly 2 note inputs"),
    (ERR_P2IDR_RECLAIM_ACCT_IS_NOT_SENDER, "P2IDR's can only be reclaimed by the sender"),
    (ERR_P2IDR_RECLAIM_HEIGHT_NOT_REACHED, "Transaction's reference block is lower than reclaim height. The P2IDR can not be reclaimed"),
    (ERR_SWAP_WRONG_NUMBER_OF_INPUTS, "SWAP script expects exactly 10 note inputs"),
    (ERR_SWAP_WRONG_NUMBER_OF_ASSETS, "SWAP script requires exactly 1 note asset"),
    (ERR_NONCE_DID_NOT_INCREASE, "The nonce did not increase after a state changing transaction"),
    (ERR_EPILOGUE_ASSETS_DONT_ADD_UP, "Total number of assets in the account and all involved notes must stay the same"),
    (ERR_PROLOGUE_GLOBAL_INPUTS_MISMATCH, "The global inputs provided do not match the block hash commitment"),
    (ERR_PROLOGUE_ACCT_STORAGE_MISMATCH, "The account storage data does not match its commitment"),
    (ERR_PROLOGUE_ACCT_STORAGE_ARITY_TOO_HIGH, "Data store in account's storage exceeds the maximum capacity of 256 elements"),
    (ERR_PROLOGUE_ACCT_STORAGE_MODE_INVALID, "Data store in account's storage contains invalid type discriminant"),
    (ERR_PROLOGUE_NEW_ACCT_VAULT_NOT_EMPTY, "New account must have an empty vault"),
    (ERR_PROLOGUE_NEW_ACCT_INVALID_SLOT_TYPE, "New account must have valid slot types"),
    (ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_NON_EMPTY_RESERVED_SLOT, "Reserved slot for new fungible faucet is not empty"),
    (ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_NON_ZERO_RESERVED_SLOT, "Reserved slot for new fungible faucet has a non-zero arity"),
    (ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_INVALID_TYPE_RESERVED_SLOT, "Reserved slot for new fungible faucet has an invalid type"),
    (ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_INVALID_RESERVED_SLOT, "Reserved slot for non-fungible faucet is not a valid empty SMT"),
    (ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_NON_ZERO_RESERVED_SLOT, "Reserved slot for new non-fungible faucet has a non-zero arity"),
    (ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_INVALID_TYPE_RESERVED_SLOT, "Reserved slot for new non-fungible faucet has an invalid type"),
    (ERR_PROLOGUE_ACCT_HASH_MISMATCH, "Account data provided does not match the commitment recorded on-chain"),
    (ERR_PROLOGUE_OLD_ACCT_NONCE_ZERO, "Existing account must have a non-zero nonce"),
    (ERR_PROLOGUE_ACCT_ID_MISMATCH, "Provided account ids via global inputs and advice provider do not match"),
    (ERR_PROLOGUE_NOTE_MMR_DIGEST_MISMATCH, "Reference block MMR and note's authentication MMR must match"),
    (ERR_NOTE_TOO_MANY_INPUTS, "Number of note inputs exceeded the maximum limit of 128"),
    (ERR_PROLOGUE_NOTE_TOO_MANY_ASSETS, "Number of note assets exceeded the maximum limit of 256"),
    (ERR_PROLOGUE_NOTE_CONSUMED_ASSETS_MISMATCH, "Provided info about assets of an input do not match its commitment"),
    (ERR_PROLOGUE_TOO_MANY_INPUT_NOTES, "Number of input notes exceeded the kernel's maximum limit of 1024"),
    (ERR_PROLOGUE_INPUT_NOTES_COMMITMENT_MISMATCH, "Commitment computed for input notes' from advice data doesn't match kernel inputs"),
    (ERR_TX_OUTPUT_NOTES_OVERFLOW, "Output notes exceeded the maximum limit of 1024"),
    (ERR_BASIC_FUNGIBLE_MAX_SUPPLY_OVERFLOW, "Distribute would cause the max supply to be exceeded"),
    (ERR_FAUCET_ISSUANCE_OVERFLOW, "Asset mint operation would cause an issuance overflow"),
    (ERR_FAUCET_BURN_OVER_ISSUANCE, "Asset burn can not exceed the existing supply"),
    (ERR_FAUCET_NON_FUNGIBLE_ALREADY_EXISTS, "Non-fungible token already exists, it can be issued only once"),
    (ERR_FAUCET_NON_FUNGIBLE_BURN_WRONG_TYPE, "Non-fungible burn called on the wrong faucet type"),
    (ERR_FAUCET_NONEXISTING_TOKEN, "Burn called on nonexistent token"),
    (ERR_NOTE_INVALID_SENDER, "Trying to access note sender from incorrect context"),
    (ERR_NOTE_INVALID_ASSETS, "Trying to access note assets from incorrect context"),
    (ERR_NOTE_INVALID_INPUTS, "Trying to access note inputs from incorrect context"),
    (ERR_NOTE_TOO_MANY_ASSETS, "Assets in a note must fit in a u8 value"),
    (ERR_VAULT_GET_BALANCE_WRONG_ASSET_TYPE, "The get_balance procedure can be called only with a fungible faucet"),
    (ERR_VAULT_HAS_NON_FUNGIBLE_WRONG_ACCOUNT_TYPE, "The has_non_fungible_asset procedure can be called only with a non-fungible faucet"),
    (ERR_VAULT_FUNGIBLE_MAX_AMOUNT_EXCEEDED, "Adding the fungible asset would exceed the max_amount of 9223372036854775807"),
    (ERR_VAULT_ADD_FUNGIBLE_ASSET_MISMATCH, "Adding the asset to the account vault failed, something is wrong with the current value before the update"),
    (ERR_VAULT_NON_FUNGIBLE_ALREADY_EXISTS, "The non-fungible asset already exists, can not be added again"),
    (ERR_VAULT_FUNGIBLE_AMOUNT_UNDERFLOW, "Removing the fungible asset results in an underflow or negative balance"),
    (ERR_VAULT_REMOVE_FUNGIBLE_ASSET_MISMATCH, "Removing the asset from the account vault failed, something is wrong with the current value before the update"),
    (ERR_VAULT_NON_FUNGIBLE_MISSING_ASSET, "Removing inexistent non-fungible asset"),
    (ERR_FUNGIBLE_ASSET_FORMAT_POSITION_ONE_MUST_BE_ZERO, "Malformed fungible asset; ASSET[1] must be 0"),
    (ERR_FUNGIBLE_ASSET_FORMAT_POSITION_TWO_MUST_BE_ZERO, "Malformed fungible asset; ASSET[2] must be 0"),
    (ERR_FUNGIBLE_ASSET_FORMAT_POSITION_THREE_MUST_BE_FUNGIBLE_FAUCET_ID, "Malformed fungible asset; ASSET[3] must be a valide fungible faucet id"),
    (ERR_FUNGIBLE_ASSET_FORMAT_POSITION_ZERO_MUST_BE_WITHIN_LIMITS, "Malformed fungible asset; ASSET[0] exceeds the maximum allowed amount"),
    (ERR_NON_FUNGIBLE_ASSET_FORMAT_POSITION_ONE_MUST_BE_FUNGIBLE_FAUCET_ID, "Malformed non-fungible asset; ASSET[1] is not a valid non-fungible faucet id"),
    (ERR_NON_FUNGIBLE_ASSET_FORMAT_HIGH_BIT_MUST_BE_ZERO, "Malformed non-fungible asset; the most significant bit must be 0"),
    (ERR_FUNGIBLE_ASSET_MISMATCH, "Fungible asset origin validation failed"),
    (ERR_NON_FUNGIBLE_ASSET_MISMATCH, "Non-fungible asset origin validation failed"),
    (ERR_ACCOUNT_NONCE_INCR_MUST_BE_U32, "The nonce cannot be increased by a greater than u32 value"),
    (ERR_ACCOUNT_INSUFFICIENT_ONES, "Account id is invalid, insufficient 1's"),
    (ERR_ACCOUNT_SET_CODE_ACCOUNT_MUST_BE_UPDATABLE, "Account must be updatable for it to be possible to update its code"),
    (ERR_ACCOUNT_SEED_DIGEST_MISMATCH, "Account seed digest mismatch"),
    (ERR_ACCOUNT_INVALID_POW, "Account pow is insufficient"),
    (ERR_NOTE_DATA_MISMATCH, "Provided note data does not match the commitment"),
    (ERR_ASSET_NOT_FUNGIBLE_ID, "Can not build the fungible asset because provided id is not a fungible id"),
    (ERR_ASSET_INVALID_AMOUNT, "Can not build the asset because amount exceeds the maximum"),
    (ERR_ASSET_NOT_NON_FUNGIBLE_ID, "Can not build the non-fungible asset because provided id is not a non-fungible id"),
    (ERR_INVALID_NOTE_TYPE, "Invalid note type"),
    (ERR_INVALID_NOTE_IDX, "Invalid note index"),
    (ERR_NOTE_INVALID_TYPE_FOR_TAG, "Provided note tag type is invalid"),
    (ERR_NOTE_FUNGIBLE_MAX_AMOUNT_EXCEEDED, "Adding a fungible asset to a note cannot exceed the maximum_amount"),
    (ERR_NON_FUNGIBLE_ASSET_ALREADY_EXISTS, "Non-fungible asset that already exists in the note cannot be added again"),
    (ERR_NOTE_TAG_MUST_BE_U32, "The note's tag high bits must be set to 0"),
    (ERR_SETTING_NON_VALUE_ITEM_ON_VALUE_SLOT, "Setting a non-value item on a value slot"),
    (ERR_SETTING_MAP_ITEM_ON_NON_MAP_SLOT, "Setting a map item on a non-map slot"),
    (ERR_READING_MAP_VALUE_FROM_NON_MAP_SLOT, "Slot type is not a map"),
    (ERR_PROC_NOT_PART_OF_ACCOUNT_CODE, "Provided procedure is not part of account code"),
    (ERR_PROC_INDEX_OUT_OF_BOUNDS, "Provided procedure index is out of bounds"),
    (ERR_ACCT_CODE_HASH_MISMATCH, "Provided account hash does not match stored account hash"),
    (ERR_ACCT_TOO_MANY_PROCEDURES, "Number of account procedures exceeded the maximum limit of 256"),
    (ERR_STORAGE_SLOT_INDEX_OUT_OF_BOUNDS, "Provided storage slot index is out of bounds"),
    (ERR_INVALID_FAUCET_STORAGE_OFFSET, "Storage offset is invalid for a faucet account (0 is prohibited being the reserved faucet data slot)"),
    (ERR_KERNEL_PROCEDURE_OFFSET_OUT_OF_BOUNDS, "Provided kernel procedure offset is out of bounds"),
    (ERR_INVALID_STORAGE_OFFSET_FOR_SIZE, "Storage offset is invalid for a procedure that does not access storage (should be 0)"),
    (ERR_CURRENT_ACCOUNT_IS_NOT_NATIVE, "Procedure can be called only for the native account"),
    (ERR_INVALID_TX_EXPIRATION_DELTA, "Invalid transaction expiration block delta was set."),
    (ERR_FOREIGN_ACCOUNT_ID_IS_ZERO, "Provided ID of the foreign account equals zero"),
    (ERR_MAX_NUM_FOREIGN_ACCOUNTS_EXCEEDED, "Maximum number of the foreign accounts (64) was exceeded"),
    (ERR_ACCT_TOO_MANY_STORAGE_SLOTS, "Number of account storage slots exceeded the maximum limit of 255"),
    (ERR_FOREIGN_ACCT_ID_EQUALS_NATIVE_ACCT_ID, "Provided foreign account ID is equal to the native account ID"),
];
