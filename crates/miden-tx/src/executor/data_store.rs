#[cfg(feature = "async")]
use alloc::boxed::Box;
use alloc::collections::BTreeSet;

use miden_objects::{
    account::{Account, AccountId},
    block::{BlockHeader, BlockNumber},
    transaction::ChainMmr,
};
use vm_processor::Word;
use winter_maybe_async::*;

use crate::DataStoreError;

// DATA STORE TRAIT
// ================================================================================================

/// The [DataStore] trait defines the interface that transaction objects use to fetch data
/// required for transaction execution.
#[maybe_async_trait]
pub trait DataStore {
    /// Returns blockchain-related data required to execute a transaction against a specific
    /// account, that consumes specific notes.
    ///
    /// The returned [`ChainMmr`] is expected to contain the complete set of requested
    /// block numbers (`ref_blocks`).
    ///
    /// # Errors
    /// Returns an error if:
    /// - The block with the specified number could not be found in the data store.
    /// - The combination of specified inputs resulted in a transaction input error.
    /// - The data store encountered some internal error
    #[maybe_async]
    fn get_chain_inputs(
        &self,
        ref_blocks: BTreeSet<BlockNumber>,
        block_header: BlockNumber,
    ) -> Result<(ChainMmr, BlockHeader), DataStoreError>;

    /// Returns account data required to execute a transaction.
    ///
    /// For a new [`Account`], the corresponding seed should be returned as the second element
    /// of the return tuple.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The account with the specified ID could not be found in the data store.
    /// - The data store encountered some internal error.
    #[maybe_async]
    fn get_account_inputs(
        &self,
        account_id: AccountId,
    ) -> Result<(Account, Option<Word>), DataStoreError>;
}
