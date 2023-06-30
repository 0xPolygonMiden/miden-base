use super::{
    Account, AccountId, BlockHeader, ChainMmr, DataStore, DataStoreError, Note, NoteOrigin,
    TransactionExecutor,
};
use crypto::StarkField;
use miden_objects::{mock::mock_inputs, transaction::TransactionOutputs, TryFromVmResult};
use processor::MemAdviceProvider;

#[derive(Clone)]
pub struct MockDataStore {
    pub account: Account,
    pub block_header: BlockHeader,
    pub block_chain: ChainMmr,
    pub notes: Vec<Note>,
}

impl MockDataStore {
    pub fn new() -> Self {
        let (account, block_header, block_chain, notes) = mock_inputs();
        Self {
            account,
            block_header,
            block_chain,
            notes,
        }
    }
}

impl Default for MockDataStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DataStore for MockDataStore {
    fn get_transaction_data(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteOrigin],
    ) -> Result<(Account, BlockHeader, ChainMmr, Vec<Note>), DataStoreError> {
        assert_eq!(account_id, self.account.id());
        assert_eq!(block_num as u64, self.block_header.block_num().as_int());
        assert_eq!(notes.len(), self.notes.len());
        let origins = self
            .notes
            .iter()
            .map(|note| note.proof().as_ref().unwrap().origin())
            .collect::<Vec<_>>();
        notes.iter().all(|note| origins.contains(&note));
        Ok((
            self.account.clone(),
            self.block_header.clone(),
            self.block_chain.clone(),
            self.notes.clone(),
        ))
    }

    fn get_account_code(
        &self,
        account_id: AccountId,
    ) -> Result<assembly::ast::ModuleAst, DataStoreError> {
        assert_eq!(account_id, self.account.id());
        Ok(self.account.code().module().clone())
    }
}

#[test]
fn test_transaction_executor_witness() {
    let data_store = MockDataStore::new();
    let mut executor = TransactionExecutor::new(data_store.clone());

    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins = data_store
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(account_id, block_ref, &note_origins, None)
        .unwrap();
    let witness = transaction_result.clone().into_witness();

    // use the witness to execute the transaction again
    let mut mem_advice_provider: MemAdviceProvider = witness.advice_inputs().clone().into();
    let result =
        processor::execute(witness.program(), witness.get_stack_inputs(), &mut mem_advice_provider)
            .unwrap();

    let reexecution_result =
        TransactionOutputs::try_from_vm_result(result.stack_outputs(), &mem_advice_provider)
            .unwrap();

    assert_eq!(transaction_result.final_account_stub(), &reexecution_result.final_account_stub);
    assert_eq!(transaction_result.created_notes(), &reexecution_result.created_notes);
}
