use super::{
    Account, BlockHeader, InputNote, InputNotes, Program, TransactionArgs, TransactionInputs,
};

// PREPARED TRANSACTION
// ================================================================================================

/// A struct that contains all of the data required to execute a transaction.
///
/// This includes:
/// - An executable program which defines the transaction.
/// - An optional transaction script.
/// - A set of inputs against which the transaction program should be executed.
#[derive(Debug)]
pub struct PreparedTransaction {
    program: Program,
    tx_inputs: TransactionInputs,
    tx_args: TransactionArgs,
}

impl PreparedTransaction {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new [PreparedTransaction] instantiated from the provided executable transaction
    /// program and inputs required to execute this program.
    pub fn new(program: Program, tx_inputs: TransactionInputs, tx_args: TransactionArgs) -> Self {
        Self { program, tx_inputs, tx_args }
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the transaction program.
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Returns the account for this transaction.
    pub fn account(&self) -> &Account {
        self.tx_inputs.account()
    }

    /// Returns the block header for this transaction.
    pub fn block_header(&self) -> &BlockHeader {
        self.tx_inputs.block_header()
    }

    /// Returns the notes to be consumed in this transaction.
    pub fn input_notes(&self) -> &InputNotes<InputNote> {
        self.tx_inputs.input_notes()
    }

    /// Returns a reference to the transaction args.
    pub fn tx_args(&self) -> &TransactionArgs {
        &self.tx_args
    }

    /// Returns a reference to the inputs for this transaction.
    pub fn tx_inputs(&self) -> &TransactionInputs {
        &self.tx_inputs
    }

    // CONVERSIONS
    // --------------------------------------------------------------------------------------------

    /// Consumes the prepared transaction and returns its parts.
    pub fn into_parts(self) -> (Program, TransactionInputs, TransactionArgs) {
        (self.program, self.tx_inputs, self.tx_args)
    }
}
