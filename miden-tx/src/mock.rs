use super::{
    Account, AccountId, BlockHeader, ChainMmr, DataStore, DataStoreError, Note, NoteOrigin,
};
use crypto::StarkField;
use miden_objects::mock::mock_inputs;

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
        note_origins: &[NoteOrigin],
    ) -> Result<(Account, BlockHeader, ChainMmr, Vec<Note>), DataStoreError> {
        assert_eq!(account_id, self.account.id());
        assert_eq!(block_num as u64, self.block_header.block_num().as_int());
        let notes = note_origins
            .iter()
            .map(|origin| {
                self.notes
                    .iter()
                    .find(|x| x.proof().as_ref().unwrap().origin() == origin)
                    .cloned()
                    .ok_or(DataStoreError::NoteNotFound(
                        origin.block_num.as_int() as u32,
                        origin.node_index,
                    ))
            })
            .collect::<Result<Vec<_>, DataStoreError>>()?;
        Ok((self.account.clone(), self.block_header.clone(), self.block_chain.clone(), notes))
    }

    fn get_account_code(
        &self,
        account_id: AccountId,
    ) -> Result<assembly::ast::ModuleAst, DataStoreError> {
        assert_eq!(account_id, self.account.id());
        Ok(self.account.code().module().clone())
    }
}
