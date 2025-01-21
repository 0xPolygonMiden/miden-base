use miden_objects::{
    accounts::{AccountHeader, AccountId},
    AccountError, Felt, WORD_SIZE,
};

use super::memory::{
    ACCT_CODE_COMMITMENT_OFFSET, ACCT_DATA_MEM_SIZE, ACCT_ID_AND_NONCE_OFFSET, ACCT_NONCE_IDX,
    ACCT_STORAGE_COMMITMENT_OFFSET, ACCT_VAULT_ROOT_OFFSET,
};
use crate::transaction::memory::{ACCT_ID_PREFIX_IDX, ACCT_ID_SUFFIX_IDX};

// STACK OUTPUTS
// ================================================================================================

/// The index of the word at which the final account nonce is stored on the output stack.
pub const OUTPUT_NOTES_COMMITMENT_WORD_IDX: usize = 0;

/// The index of the word at which the final account hash is stored on the output stack.
pub const FINAL_ACCOUNT_HASH_WORD_IDX: usize = 1;

/// The index of the item at which the expiration block height is stored on the output stack.
pub const EXPIRATION_BLOCK_ELEMENT_IDX: usize = 8;

// ACCOUNT HEADER EXTRACTOR
// ================================================================================================

/// Parses the account header data returned by the VM into individual account component commitments.
/// Returns a tuple of account ID, vault root, storage commitment, code commitment, and nonce.
pub fn parse_final_account_header(elements: &[Felt]) -> Result<AccountHeader, AccountError> {
    if elements.len() != ACCT_DATA_MEM_SIZE {
        return Err(AccountError::HeaderDataIncorrectLength {
            actual: elements.len(),
            expected: ACCT_DATA_MEM_SIZE,
        });
    }

    let id = AccountId::try_from([
        elements[ACCT_ID_AND_NONCE_OFFSET as usize + ACCT_ID_PREFIX_IDX],
        elements[ACCT_ID_AND_NONCE_OFFSET as usize + ACCT_ID_SUFFIX_IDX],
    ])
    .map_err(AccountError::FinalAccountHeaderIdParsingFailed)?;
    let nonce = elements[ACCT_ID_AND_NONCE_OFFSET as usize + ACCT_NONCE_IDX];
    let vault_root = TryInto::<[Felt; 4]>::try_into(
        &elements[ACCT_VAULT_ROOT_OFFSET as usize..ACCT_VAULT_ROOT_OFFSET as usize + WORD_SIZE],
    )
    .unwrap()
    .into();
    let storage_commitment = TryInto::<[Felt; 4]>::try_into(
        &elements[ACCT_STORAGE_COMMITMENT_OFFSET as usize
            ..ACCT_STORAGE_COMMITMENT_OFFSET as usize + WORD_SIZE],
    )
    .unwrap()
    .into();
    let code_commitment = TryInto::<[Felt; 4]>::try_into(
        &elements[ACCT_CODE_COMMITMENT_OFFSET as usize
            ..ACCT_CODE_COMMITMENT_OFFSET as usize + WORD_SIZE],
    )
    .unwrap()
    .into();

    Ok(AccountHeader::new(id, nonce, vault_root, storage_commitment, code_commitment))
}
