use miden_objects::{
    accounts::{Account, AccountId, AccountStorage, AccountStorageDelta, AccountStub},
    crypto::merkle::{merkle_tree_delta, MerkleStore},
    transaction::FinalAccountStub,
    AccountError, TransactionResultError, Word,
};

use crate::memory::{
    ACCT_CODE_ROOT_OFFSET, ACCT_DATA_MEM_SIZE, ACCT_ID_AND_NONCE_OFFSET, ACCT_ID_IDX,
    ACCT_NONCE_IDX, ACCT_STORAGE_ROOT_OFFSET, ACCT_VAULT_ROOT_OFFSET,
};

/// Parses the stub account data returned by the VM into individual account component commitments.
/// Returns a tuple of account ID, vault root, storage root, code root, and nonce.
pub fn parse_final_account_stub(elements: &[Word]) -> Result<AccountStub, AccountError> {
    if elements.len() != ACCT_DATA_MEM_SIZE {
        return Err(AccountError::StubDataIncorrectLength(elements.len(), ACCT_DATA_MEM_SIZE));
    }

    let id = AccountId::try_from(elements[ACCT_ID_AND_NONCE_OFFSET as usize][ACCT_ID_IDX])?;
    let nonce = elements[ACCT_ID_AND_NONCE_OFFSET as usize][ACCT_NONCE_IDX];
    let vault_root = elements[ACCT_VAULT_ROOT_OFFSET as usize].into();
    let storage_root = elements[ACCT_STORAGE_ROOT_OFFSET as usize].into();
    let code_root = elements[ACCT_CODE_ROOT_OFFSET as usize].into();

    Ok(AccountStub::new(id, nonce, vault_root, storage_root, code_root))
}

// ACCOUNT STORAGE DELTA
// ================================================================================================
/// Extracts account storage delta between the `initial_account` and `final_account_stub` from the
/// provided `MerkleStore`
pub fn extract_account_storage_delta(
    store: &MerkleStore,
    initial_account: &Account,
    final_account_stub: &FinalAccountStub,
) -> Result<AccountStorageDelta, TransactionResultError> {
    // extract storage slots delta
    let tree_delta = merkle_tree_delta(
        initial_account.storage().root(),
        final_account_stub.0.storage_root(),
        AccountStorage::STORAGE_TREE_DEPTH,
        store,
    )
    .map_err(TransactionResultError::ExtractAccountStorageSlotsDeltaFailed)?;

    // map tree delta to cleared/updated slots; we can cast indexes to u8 because the
    // the number of storage slots cannot be greater than 256
    let cleared_items = tree_delta.cleared_slots().iter().map(|idx| *idx as u8).collect();
    let updated_items = tree_delta
        .updated_slots()
        .iter()
        .map(|(idx, value)| (*idx as u8, *value))
        .collect();

    // construct storage delta
    let storage_delta = AccountStorageDelta { cleared_items, updated_items };

    Ok(storage_delta)
}
