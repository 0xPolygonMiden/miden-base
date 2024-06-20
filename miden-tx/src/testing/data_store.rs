use alloc::vec::Vec;

use miden_objects::{
    accounts::{
        account_id::testing::ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN, Account,
        AccountId,
    },
    assembly::ModuleAst,
    notes::{Note, NoteId},
    testing::notes::AssetPreservationStatus,
    transaction::{InputNote, InputNotes, TransactionArgs, TransactionInputs},
    BlockHeader,
};
use vm_processor::ONE;
use winter_maybe_async::maybe_async;

use super::TransactionContextBuilder;
use crate::{DataStore, DataStoreError};

// MOCK DATA STORE
// ================================================================================================

#[derive(Clone)]
pub struct MockDataStore {
    pub tx_inputs: TransactionInputs,
    pub tx_args: TransactionArgs,
}

impl MockDataStore {
    pub fn new(asset_preservation_status: AssetPreservationStatus) -> Self {
        let tx_context = TransactionContextBuilder::with_standard_account(
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
            ONE,
        )
        .with_mock_notes(asset_preservation_status)
        .build();
        let (_, _, tx_args, tx_inputs) = tx_context.into_parts();
        Self { tx_inputs, tx_args }
    }

    pub fn with_existing(account: Account, input_notes: Vec<Note>) -> Self {
        let (_, _, tx_args, tx_inputs) = TransactionContextBuilder::new(account)
            .input_notes(input_notes)
            .build()
            .into_parts();

        Self { tx_inputs, tx_args }
    }

    pub fn input_notes(&self) -> &InputNotes<InputNote> {
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
    #[maybe_async]
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

    #[maybe_async]
    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        assert_eq!(account_id, self.tx_inputs.account().id());
        Ok(self.tx_inputs.account().code().module().clone())
    }
}
