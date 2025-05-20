#[cfg(feature = "async")]
use alloc::boxed::Box;
use alloc::collections::BTreeSet;

use miden_objects::{
    account::{Account, AccountId},
    block::{BlockHeader, BlockNumber},
    transaction::PartialBlockchain,
};
use vm_processor::{MastForestStore, Word};
use winter_maybe_async::*;

use crate::DataStoreError;

// DATA STORE TRAIT
// ================================================================================================

/// The [DataStore] trait defines the interface that transaction objects use to fetch data
/// required for transaction execution.
#[maybe_async_trait]
pub trait DataStore: MastForestStore + Send + Sync {
    /// Returns all the data required to execute a transaction against the account with the
    /// specified ID and consuming input notes created in blocks in the input `ref_blocks` set.
    ///
    /// The highest block number in `ref_blocks` will be the transaction reference block. In
    /// general, it is recommended that the reference corresponds to the latest block available
    /// in the data store.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The account with the specified ID could not be found in the data store.
    /// - The block with the specified number could not be found in the data store.
    /// - The combination of specified inputs resulted in a transaction input error.
    /// - The data store encountered some internal error
    #[maybe_async]
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        ref_blocks: BTreeSet<BlockNumber>,
    ) -> Result<(Account, Option<Word>, BlockHeader, PartialBlockchain), DataStoreError>;
}
