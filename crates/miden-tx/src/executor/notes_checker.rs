use alloc::vec::Vec;

use miden_lib::{
    account::interface::NoteAccountCompatibility, note::well_known_note::WellKnownNote,
};
use miden_objects::{
    account::AccountId,
    block::BlockNumber,
    note::NoteId,
    transaction::{InputNote, TransactionArgs},
};
use winter_maybe_async::{maybe_async, maybe_await};

use super::{NoteAccountExecution, TransactionExecutor, TransactionExecutorError};

/// This struct performs input notes checks against provided target account.
///
/// A check could be:
/// - Static -- performed by the [NotesChecker::check_note_inputs]. See method description.
/// - Dynamic -- performed by the [NotesChecker::check_notes_consumability]. Essentially runs the
///   transaction to make sure that provided input notes could be consumed by the account.
pub struct NotesChecker {
    account_id: AccountId,
    notes: Vec<InputNote>,
}

impl NotesChecker {
    /// Returns a new instance of the [NotesChecker].
    pub fn new(account_id: AccountId, notes: Vec<InputNote>) -> Self {
        NotesChecker { account_id, notes }
    }

    /// Checks whether there are "well known" notes (`P2ID`, `P2IDR` and `SWAP`) in the list of the
    /// provided input notes. If so, assert that the note inputs are correct.
    ///
    /// Returns [NoteAccountCompatibility::No] if at least one note has incorrect inputs.
    pub fn check_note_inputs(&self) -> NoteInputsCheck {
        for note in self.notes.iter() {
            if let Some(well_known_note) = WellKnownNote::from_note(note.note()) {
                if let NoteAccountCompatibility::No =
                    well_known_note.check_note_inputs(note.note(), self.account_id)
                {
                    return NoteInputsCheck::No { failed_note_id: note.id() };
                }
            }
        }

        NoteInputsCheck::Maybe
    }

    /// Checks whether the provided input notes could be consumed by the provided account.
    ///
    /// This check consists of two main steps:
    /// - Check whether there are "well known" notes (`P2ID`, `P2IDR` and `SWAP`) in the list of the
    ///   provided input notes. If so, assert that the note inputs are correct.
    /// - Execute the transaction with specified notes.
    ///   - Returns `NoteAccountExecution::Success` if the execution was successful.
    ///   - Returns `NoteAccountExecution::Failure` if some note returned an error. The fields
    ///     associated with `Failure` variant contains the ID of the failed note, a vector of IDs of
    ///     the notes, which were successfully executed, and the [TransactionExecutorError] if the
    ///     check failed durning the execution stage.
    #[maybe_async]
    pub fn check_notes_consumability(
        &self,
        tx_executor: &TransactionExecutor,
        block_ref: BlockNumber,
        tx_args: TransactionArgs,
    ) -> Result<NoteAccountExecution, TransactionExecutorError> {
        // Check input notes
        // ----------------------------------------------------------------------------------------
        let inputs_check_result = self.check_note_inputs();
        if let NoteInputsCheck::No { failed_note_id } = inputs_check_result {
            return Ok(NoteAccountExecution::Failure {
                failed_note_id,
                successful_notes: vec![],
                error: None,
            });
        }

        // Execute transaction
        // ----------------------------------------------------------------------------------------
        let note_ids = self.notes.iter().map(|note| note.id()).collect::<Vec<NoteId>>();
        maybe_await!(tx_executor.try_notes_execution(
            self.account_id,
            block_ref,
            &note_ids,
            tx_args
        ))
    }
}

/// Helper enum for getting a result of the well known note inputs check.
#[derive(Debug, PartialEq)]
pub enum NoteInputsCheck {
    Maybe,
    No { failed_note_id: NoteId },
}
