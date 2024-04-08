use crate::MockDataStore;
use miden_tx::TransactionExecutor;

#[test]
fn benchmark_default_tx() {
    let data_store = MockDataStore::default();
    let mut executor = TransactionExecutor::new(data_store.clone());

    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    executor.execute_transaction(account_id, block_ref, &note_ids, None).unwrap();
}
