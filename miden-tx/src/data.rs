use miden_objects::{assembly::ModuleAst, transaction::TransactionInputs};

use super::{AccountId, DataStoreError, NoteOrigin};

// DATA STORE TRAIT
// ================================================================================================

/// The [DataStore] trait defines the interface that transaction objects use to fetch data
/// required for transaction execution.
pub trait DataStore {
    /// Returns account, chain, and input note data required to execute a transaction against
    /// the account with the specified ID and consuming the set of specified input notes.
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteOrigin],
    ) -> Result<TransactionInputs, DataStoreError>;

    /// Returns the account code [ModuleAst] associated with the the specified [AccountId].
    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError>;
}
