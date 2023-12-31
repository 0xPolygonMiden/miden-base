use super::{Account, BlockHeader, InputNotes, Program, TransactionInputs, TransactionScript};
use crate::TransactionError;

// PREPARED TRANSACTION
// ================================================================================================

/// A struct that contains all of the data required to execute a transaction.
///
/// This includes:
/// - A an executable program which defines the transaction.
/// - An optional transaction script.
/// - A set of inputs against which the transaction program should be executed.
#[derive(Debug)]
pub struct PreparedTransaction {
    program: Program,
    tx_script: Option<TransactionScript>,
    tx_inputs: TransactionInputs,
}

impl PreparedTransaction {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new [PreparedTransaction] instantiated from the provided executable transaction
    /// program and inputs required to execute this program.
    ///
    /// # Returns an error if:
    /// - For a new account, account seed is not provided or the provided seed is invalid.
    /// - For an existing account, account seed was provided.
    pub fn new(
        program: Program,
        tx_script: Option<TransactionScript>,
        tx_inputs: TransactionInputs,
    ) -> Result<Self, TransactionError> {
        tx_inputs.validate_new_account_seed()?;
        Ok(Self { program, tx_script, tx_inputs })
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the transaction program.
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Returns the account for this transaction.
    pub fn account(&self) -> &Account {
        &self.tx_inputs.account
    }

    /// Returns the block header for this transaction.
    pub fn block_header(&self) -> &BlockHeader {
        &self.tx_inputs.block_header
    }

    /// Returns the notes to be consumed in this transaction.
    pub fn input_notes(&self) -> &InputNotes {
        &self.tx_inputs.input_notes
    }

    /// Return a reference the transaction script.
    pub fn tx_script(&self) -> Option<&TransactionScript> {
        self.tx_script.as_ref()
    }

    /// Returns a reference to the inputs for this transaction.
    pub fn tx_inputs(&self) -> &TransactionInputs {
        &self.tx_inputs
    }

    // CONVERSIONS
    // --------------------------------------------------------------------------------------------

    /// Consumes the prepared transaction and returns its parts.
    pub fn into_parts(self) -> (Program, Option<TransactionScript>, TransactionInputs) {
        (self.program, self.tx_script, self.tx_inputs)
    }
}
