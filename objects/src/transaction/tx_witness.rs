use super::{
    Account, AdviceInputs, BlockHeader, InputNote, InputNotes, Program, TransactionArgs,
    TransactionInputs,
};

// TRANSACTION WITNESS
// ================================================================================================

/// Transaction witness contains all the data required to execute and prove a Miden rollup
/// transaction.
///
/// The main purpose of the transaction witness is to enable stateless re-execution and proving
/// of transactions.
///
/// A transaction witness consists of:
/// - The executable transaction [Program].
/// - Transaction inputs which contain information about the initial state of the account, input
///   notes, block header etc.
/// - An optional transaction script.
/// - Advice witness which contains all data requested by the VM from the advice provider while
///   executing the transaction program.
///
/// TODO: currently, the advice witness contains redundant and irrelevant data (e.g., tx inputs
/// and tx outputs). we should optimize it to contain only the minimum data required for
/// executing/proving the transaction.
pub struct TransactionWitness {
    program: Program,
    tx_inputs: TransactionInputs,
    tx_args: TransactionArgs,
    advice_witness: AdviceInputs,
}

impl TransactionWitness {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates a new [TransactionWitness] from the provided data.
    pub fn new(
        program: Program,
        tx_inputs: TransactionInputs,
        tx_args: TransactionArgs,
        advice_witness: AdviceInputs,
    ) -> Self {
        Self {
            program,
            tx_inputs,
            tx_args,
            advice_witness,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a reference the program defining this transaction.
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Returns the account state before the transaction was executed.
    pub fn account(&self) -> &Account {
        self.tx_inputs.account()
    }

    /// Returns the notes consumed in this transaction.
    pub fn input_notes(&self) -> &InputNotes<InputNote> {
        self.tx_inputs.input_notes()
    }

    /// Returns the block header for the block against which the transaction was executed.
    pub fn block_header(&self) -> &BlockHeader {
        self.tx_inputs.block_header()
    }

    /// Returns a reference to the transaction args.
    pub fn tx_args(&self) -> &TransactionArgs {
        &self.tx_args
    }

    /// Returns a reference to the inputs for this transaction.
    pub fn tx_inputs(&self) -> &TransactionInputs {
        &self.tx_inputs
    }

    /// Returns all the data requested by the VM from the advice provider while executing the
    /// transaction program.
    pub fn advice_witness(&self) -> &AdviceInputs {
        &self.advice_witness
    }
}
