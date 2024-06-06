// MOCK DATA STORE
// ================================================================================================

use alloc::vec::Vec;

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountId},
    assembly::ModuleAst,
    notes::{Note, NoteId},
    testing::{account::MockAccountType, notes::AssetPreservationStatus},
    transaction::{InputNotes, TransactionArgs, TransactionInputs},
    BlockHeader,
};

use super::TransactionContextBuilder;
use crate::{DataStore, DataStoreError};

#[derive(Clone)]
pub struct MockDataStore {
    pub tx_inputs: TransactionInputs,
    pub tx_args: TransactionArgs,
}

impl MockDataStore {
    pub fn new(asset_preservation_status: AssetPreservationStatus) -> Self {
        let tx_context =
            TransactionContextBuilder::with_acc_type(MockAccountType::StandardExisting)
                .with_mock_notes(asset_preservation_status)
                .build();
        let (_, _, tx_args, tx_inputs) = tx_context.into_parts();
        Self { tx_inputs, tx_args }
    }

    pub fn with_existing(account: Option<Account>, input_notes: Option<Vec<Note>>) -> Self {
        let tx_context = if let Some(acc) = account {
            TransactionContextBuilder::new(acc, TransactionKernel::assembler())
        } else {
            TransactionContextBuilder::with_acc_type(MockAccountType::StandardExisting)
        };

        let tx_context = if let Some(notes) = input_notes {
            tx_context.notes(notes)
        } else {
            tx_context.with_mock_notes(AssetPreservationStatus::Preserved)
        };
        let (_, _, tx_args, tx_inputs) = tx_context.build().into_parts();

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
