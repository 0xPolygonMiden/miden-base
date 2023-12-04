// STACK OUTPUTS
// ================================================================================================

/// The index of the word at which the transaction script root is stored on the output stack.
pub const TX_SCRIPT_ROOT_WORD_IDX: usize = 0;

/// The index of the word at which the final account nonce is stored on the output stack.
pub const CREATED_NOTES_COMMITMENT_WORD_IDX: usize = 1;

/// The index of the word at which the final account hash is stored on the output stack.
pub const FINAL_ACCOUNT_HASH_WORD_IDX: usize = 2;
