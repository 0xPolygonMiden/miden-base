use miden_objects::{assembly::ModuleAst, notes::RecordedNote};
use vm_processor::AdviceInputs;

use super::{Account, AccountId, BlockHeader, ChainMmr, DataStoreError, NoteOrigin};

/// The [DataStore] trait defines the interface that transaction objects use to fetch data
/// required for transaction execution.
pub trait DataStore {
    /// Returns the [Account], [BlockHeader], [ChainMmr], [RecordedNote]s and [AdviceInputs] required
    /// for transaction execution.
    fn get_transaction_data(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteOrigin],
    ) -> Result<(Account, BlockHeader, ChainMmr, Vec<RecordedNote>, AdviceInputs), DataStoreError>;

    /// Returns the account code [ModuleAst] associated with the the specified [AccountId].
    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError>;
}
