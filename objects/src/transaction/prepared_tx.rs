use super::{
    Account, BlockHeader, InputNotes, NoteId, Program, TransactionInputs, TransactionScript, Word,
};
use crate::utils::collections::BTreeMap;

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
    note_args: Option<BTreeMap<NoteId, Word>>,
}

impl PreparedTransaction {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new [PreparedTransaction] instantiated from the provided executable transaction
    /// program and inputs required to execute this program.
    pub fn new(
        program: Program,
        tx_script: Option<TransactionScript>,
        tx_inputs: TransactionInputs,
        note_args: Option<BTreeMap<NoteId, Word>>,
    ) -> Self {
        Self { program, tx_script, tx_inputs, note_args }
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
    pub fn input_notes(&self) -> &InputNotes {
        self.tx_inputs.input_notes()
    }

    /// Return a reference the transaction script.
    pub fn tx_script(&self) -> Option<&TransactionScript> {
        self.tx_script.as_ref()
    }

    /// Returns a reference to the inputs for this transaction.
    pub fn tx_inputs(&self) -> &TransactionInputs {
        &self.tx_inputs
    }

    /// Returns a reference to the inputs for this transaction.
    pub fn note_args(&self) -> Option<&BTreeMap<NoteId, Word>> {
        self.note_args.as_ref()
    }

    // CONVERSIONS
    // --------------------------------------------------------------------------------------------

    /// Consumes the prepared transaction and returns its parts.
    pub fn into_parts(
        self,
    ) -> (
        Program,
        Option<TransactionScript>,
        TransactionInputs,
        Option<BTreeMap<NoteId, Word>>,
    ) {
        (self.program, self.tx_script, self.tx_inputs, self.note_args)
    }
}
