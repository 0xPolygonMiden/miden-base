use super::{Account, BlockHeader, ChainMmr, RecordedNote, Vec, Word};

// TRANSACTION INPUTS
// ================================================================================================

/// Contains the data required to execute a transaction.
pub struct TransactionInputs {
    pub account: Account,
    pub account_seed: Option<Word>,
    pub block_header: BlockHeader,
    pub block_chain: ChainMmr,
    pub input_notes: Vec<RecordedNote>,
}
