use super::{
    Account, BTreeMap, BlockHeader, InputNotes, NoteId, Program, TransactionArgs,
    TransactionInputs, TransactionScript, Word,
};

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
    pub fn input_notes(&self) -> &InputNotes {
        self.tx_inputs.input_notes()
    }

    /// Return a reference the transaction script.
    pub fn tx_script(&self) -> Option<&TransactionScript> {
        self.tx_args.tx_script()
    }

    /// Returns a reference to the inputs for this transaction.
    pub fn tx_inputs(&self) -> &TransactionInputs {
        &self.tx_inputs
    }

    /// Return a reference the transaction script.
    pub fn note_args(&self) -> Option<&BTreeMap<NoteId, Word>> {
        self.tx_args.note_args()
    }

    // CONVERSIONS
    // --------------------------------------------------------------------------------------------

    /// Consumes the prepared transaction and returns its parts.
    pub fn into_parts(self) -> (Program, TransactionInputs, TransactionArgs) {
        (self.program, self.tx_inputs, self.tx_args)
    }
}
