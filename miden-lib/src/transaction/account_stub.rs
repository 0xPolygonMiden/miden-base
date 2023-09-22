use crate::memory::{
    ACCT_CODE_ROOT_OFFSET, ACCT_DATA_MEM_SIZE, ACCT_ID_AND_NONCE_OFFSET, ACCT_ID_IDX,
    ACCT_NONCE_IDX, ACCT_STORAGE_ROOT_OFFSET, ACCT_VAULT_ROOT_OFFSET,
};
use crypto::{
    merkle::{merkle_tree_delta, MerkleStore, MerkleStoreDelta, NodeIndex},
    Word,
};
use miden_objects::{
    accounts::{Account, AccountId, AccountStorage, AccountStorageDelta, AccountStub},
    transaction::FinalAccountStub,
    AccountError, TransactionResultError,
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
    let slots_delta = merkle_tree_delta(
        initial_account.storage().root(),
        final_account_stub.0.storage_root(),
        AccountStorage::STORAGE_TREE_DEPTH,
        store,
    )
    .map_err(TransactionResultError::ExtractAccountStorageSlotsDeltaFailed)?;

    // extract child deltas
    let mut store_delta = vec![];
    for (slot, new_value) in slots_delta.updated_slots() {
        // if a slot was updated, check if it was originally a Merkle root of a Merkle tree
        let leaf = store
            .get_node(
                initial_account.storage().root(),
                NodeIndex::new_unchecked(AccountStorage::STORAGE_TREE_DEPTH, *slot),
            )
            .expect("storage slut must exist");
        // if a slot was a Merkle root then extract the delta.  We assume the tree is a SMT of depth 64.
        if store.get_node(leaf, NodeIndex::new_unchecked(0, 0)).is_ok() {
            let child_delta = merkle_tree_delta(leaf, (*new_value).into(), 64, store)
                .map_err(TransactionResultError::ExtractAccountStorageStoreDeltaFailed)?;
            store_delta.push((leaf, child_delta));
        }
    }

    // construct storage delta
    let storage_delta = AccountStorageDelta {
        slots_delta,
        store_delta: MerkleStoreDelta(store_delta),
    };

    Ok(storage_delta)
}
