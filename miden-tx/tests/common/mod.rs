use miden_objects::{
    accounts::{Account, AccountId},
    assembly::ModuleAst,
    notes::{Note, NoteOrigin, RecordedNote},
    BlockHeader, ChainMmr, StarkField,
};
use miden_tx::{DataStore, DataStoreError};
use mock::{
    mock::account::MockAccountType,
    mock::notes::AssetPreservationStatus,
    mock::transaction::{mock_inputs, mock_inputs_with_existing},
};

#[derive(Clone)]
pub struct MockDataStore {
    pub account: Account,
    pub block_header: BlockHeader,
    pub block_chain: ChainMmr,
    pub notes: Vec<RecordedNote>,
}

impl MockDataStore {
    pub fn new() -> Self {
        let (account, block_header, block_chain, consumed_notes) =
            mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
        Self {
            account,
            block_header,
            block_chain,
            notes: consumed_notes,
        }
    }

    pub fn with_existing(account: Option<Account>, consumed_notes: Option<Vec<Note>>) -> Self {
        let (account, block_header, block_chain, consumed_notes) = mock_inputs_with_existing(
            MockAccountType::StandardExisting,
            AssetPreservationStatus::Preserved,
            account,
            consumed_notes,
        );
        Self {
            account,
            block_header,
            block_chain,
            notes: consumed_notes,
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
    ) -> Result<(Account, BlockHeader, ChainMmr, Vec<RecordedNote>), DataStoreError> {
        assert_eq!(account_id, self.account.id());
        assert_eq!(block_num as u64, self.block_header.block_num().as_int());
        assert_eq!(notes.len(), self.notes.len());
        let origins = self.notes.iter().map(|note| note.origin()).collect::<Vec<_>>();
        notes.iter().all(|note| origins.contains(&note));
        Ok((
            self.account.clone(),
            self.block_header,
            self.block_chain.clone(),
            self.notes.clone(),
        ))
    }

    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        assert_eq!(account_id, self.account.id());
        Ok(self.account.code().module().clone())
    }
}
