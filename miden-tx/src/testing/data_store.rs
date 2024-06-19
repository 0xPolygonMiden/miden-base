// MOCK DATA STORE
// ================================================================================================

use alloc::vec::Vec;

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountId},
    assembly::ModuleAst,
    notes::{Note, NoteId},
    testing::{
        account::MockAccountType,
        account_code::mock_account_code,
        notes::{mock_notes, AssetPreservationStatus},
    },
    transaction::{
        ChainMmr, InputNote, InputNotes, OutputNote, TransactionArgs, TransactionInputs,
    },
    BlockHeader,
};
use winter_maybe_async::maybe_async;

use super::{chain_data::mock_chain_data, mock_host::mock_inputs_with_account_seed};
use crate::{DataStore, DataStoreError};

#[derive(Clone)]
pub struct MockDataStore {
    pub account: Account,
    pub block_header: BlockHeader,
    pub block_chain: ChainMmr,
    pub notes: Vec<InputNote>,
    pub tx_args: TransactionArgs,
}

impl MockDataStore {
    pub fn new(asset_preservation_status: AssetPreservationStatus) -> Self {
        let (tx_inputs, tx_args) = mock_inputs_with_account_seed(
            MockAccountType::StandardExisting,
            asset_preservation_status,
            None,
        );
        let (account, _, block_header, block_chain, notes) = tx_inputs.into_parts();
        Self {
            account,
            block_header,
            block_chain,
            notes: notes.into_vec(),
            tx_args,
        }
    }

    pub fn with_existing(account: Account, input_notes: Option<Vec<Note>>) -> Self {
        let assembler = &TransactionKernel::assembler();

        // NOTE: this function is called because of its side effects, it will modify the state of
        // the assembler, the changes are required to register the account's procedures into the
        // assembler procedure cache, which is then required to successfully compile the
        // transaction.
        let _ = mock_account_code(assembler);

        let (mut consumed_notes, created_notes) =
            mock_notes(assembler, &AssetPreservationStatus::Preserved);
        if let Some(ref notes) = input_notes {
            consumed_notes = notes.to_vec();
        }

        let (block_chain, recorded_notes) = mock_chain_data(consumed_notes);

        let block_header =
            BlockHeader::mock(4, Some(block_chain.peaks().hash_peaks()), None, &[account.clone()]);

        let output_notes = created_notes.into_iter().filter_map(|note| match note {
            OutputNote::Full(note) => Some(note),
            OutputNote::Partial(_) => None,
            OutputNote::Header(_) => None,
        });
        let mut tx_args = TransactionArgs::default();
        tx_args.extend_expected_output_notes(output_notes);

        Self {
            account,
            block_header,
            block_chain,
            notes: recorded_notes,
            tx_args,
        }
    }

    pub fn tx_args(&self) -> &TransactionArgs {
        &self.tx_args
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
        assert_eq!(account_id, self.account.id());
        assert_eq!(block_num, self.block_header.block_num());
        assert_eq!(notes.len(), self.notes.len());

        let notes = self
            .notes
            .iter()
            .filter(|note| notes.contains(&note.id()))
            .cloned()
            .collect::<Vec<_>>();

        Ok(TransactionInputs::new(
            self.account.clone(),
            None,
            self.block_header,
            self.block_chain.clone(),
            InputNotes::new(notes).unwrap(),
        )
        .unwrap())
    }

    #[maybe_async]
    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        assert_eq!(account_id, self.account.id());
        Ok(self.account.code().module().clone())
    }
}
