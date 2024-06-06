// MOCK DATA STORE
// ================================================================================================

use alloc::vec::Vec;

use miden_objects::{
    accounts::{Account, AccountId},
    assembly::ModuleAst,
    notes::{Note, NoteId},
    testing::{account::MockAccountType, notes::AssetPreservationStatus},
    transaction::{InputNotes, OutputNote, TransactionArgs, TransactionInputs},
    BlockHeader,
};

use super::mock_host::{mock_inputs, mock_inputs_with_existing};
use crate::{DataStore, DataStoreError};

#[derive(Clone)]
pub struct MockDataStore {
    pub tx_inputs: TransactionInputs,
    pub tx_args: TransactionArgs,
}

impl MockDataStore {
    pub fn new(asset_preservation_status: AssetPreservationStatus) -> Self {
        let (tx_inputs, tx_args) =
            mock_inputs(MockAccountType::StandardExisting, asset_preservation_status);
        Self { tx_inputs, tx_args }
    }

    pub fn with_existing(account: Option<Account>, input_notes: Option<Vec<Note>>) -> Self {
        let (tx_inputs, created_notes) = mock_inputs_with_existing(
            MockAccountType::StandardExisting,
            AssetPreservationStatus::Preserved,
            account,
            input_notes,
        );
        let mut tx_args = TransactionArgs::default();
        let output_notes = created_notes.into_iter().filter_map(|note| match note {
            OutputNote::Full(note) => Some(note),
            OutputNote::Partial(_) => None,
            OutputNote::Header(_) => None,
        });
        tx_args.extend_expected_output_notes(output_notes);

        Self { tx_inputs, tx_args }
    }

    pub fn input_notes(&self) -> &InputNotes {
        self.tx_inputs.input_notes()
    }

    pub fn block_header(&self) -> &BlockHeader {
        self.tx_inputs.block_header()
    }

    pub fn account(&self) -> &Account {
        self.tx_inputs.account()
    }
}

impl Default for MockDataStore {
    fn default() -> Self {
        Self::new(AssetPreservationStatus::Preserved)
    }
}

impl DataStore for MockDataStore {
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteId],
    ) -> Result<TransactionInputs, DataStoreError> {
        assert_eq!(account_id, self.tx_inputs.account().id());
        assert_eq!(block_num, self.block_header().block_num());
        assert_eq!(notes.len(), self.tx_inputs.input_notes().num_notes());

        Ok(self.tx_inputs.clone())
    }

    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        assert_eq!(account_id, self.tx_inputs.account().id());
        Ok(self.tx_inputs.account().code().module().clone())
    }
}
