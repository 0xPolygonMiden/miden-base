use super::{
    Account, AccountId, BlockHeader, ChainMmr, DataStore, DataStoreError, Note, NoteOrigin,
    TransactionExecutor,
};
use crypto::{hash::rpo::Rpo256 as Hasher, Felt, StarkField};
use processor::MemAdviceProvider;
use test_utils::data::mock_inputs;

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
    let transaction_witness = executor
        .execute_transaction(account_id, block_ref, &note_origins, None)
        .unwrap();

    // assert the transaction witness has calculates the correct consumed notes commitment
    let consumed_notes_commitment = Hasher::hash_elements(
        &transaction_witness
            .consumed_notes_info()
            .unwrap()
            .into_iter()
            .flat_map(|info| <[Felt; 8]>::from(info))
            .collect::<Vec<Felt>>(),
    );

    assert_eq!(transaction_witness.consumed_notes_info().unwrap().len(), note_origins.len());
    assert_eq!(consumed_notes_commitment, *transaction_witness.consumed_notes_hash());

    // use the witness to execute the transaction again
    let mem_advice_provider: MemAdviceProvider = transaction_witness.advice_inputs().clone().into();
    let mut _result = processor::execute(
        transaction_witness.program(),
        transaction_witness.get_stack_inputs(),
        mem_advice_provider,
    )
    .unwrap();

    // TODO: assert the results of the two transaction executions are consistent.
}
