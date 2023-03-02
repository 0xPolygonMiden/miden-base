use super::{AccountId, Digest, Note, Vec};
use miden_core::Program;

/// Resultant object of compiling a set of note scripts in the context of an account.
/// The CompiledTransaction does not contain any account data, it is passed to a component
/// that will do the required data lookups before processing the transaction further.
/// Contains:
/// - account_id: the account that the transaction was executed against.
/// - consumed_notes: a list of consumed notes.
/// - tx_script_root: optional MAST root for the transaction script.
/// - tx_program: an executable program describing the transaction.
pub struct CompiledTransaction {
    account_id: AccountId,
    consumed_notes: Vec<Note>,
    tx_script_root: Option<Digest>,
    tx_program: Program,
}

impl CompiledTransaction {
    /// Creates a new CompiledTransaction object.
    pub fn new(
        account_id: AccountId,
        consumed_notes: Vec<Note>,
        tx_script_root: Option<Digest>,
        tx_program: Program,
    ) -> Self {
        Self {
            account_id,
            consumed_notes,
            tx_script_root,
            tx_program,
        }
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the account id.
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Returns the consumed notes.
    pub fn consumed_notes(&self) -> &[Note] {
        &self.consumed_notes
    }

    /// Returns the transaction script root.
    pub fn tx_script_root(&self) -> Option<Digest> {
        self.tx_script_root
    }

    /// Returns the transaction program.
    pub fn tx_program(&self) -> &Program {
        &self.tx_program
    }
}
