use super::{AdviceInputs, TransactionArgs, TransactionInputs};

// TRANSACTION WITNESS
// ================================================================================================

/// Transaction witness contains all the data required to execute and prove a Miden rollup
/// transaction.
///
/// The main purpose of the transaction witness is to enable stateless re-execution and proving
/// of transactions.
///
/// A transaction witness consists of:
/// - Transaction inputs which contain information about the initial state of the account, input
///   notes, block header etc.
/// - Optional transaction arguments which may contain a transaction script, note arguments, and
///   any additional advice data to initialize the advice provide with prior to transaction
///   execution.
/// - Advice witness which contains all data requested by the VM from the advice provider while
///   executing the transaction program.
///
/// TODO: currently, the advice witness contains redundant and irrelevant data (e.g., tx inputs
/// and tx outputs). we should optimize it to contain only the minimum data required for
/// executing/proving the transaction.
pub struct TransactionWitness {
    pub tx_inputs: TransactionInputs,
    pub tx_args: TransactionArgs,
    pub advice_witness: AdviceInputs,
}
