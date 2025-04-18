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

/// This struct performs input notes check against provided target account.
///
/// The check is performed using the [NotesChecker::check_notes_consumability] procedure.
/// Essentially runs the transaction to make sure that provided input notes could be consumed by the
/// account.
pub struct NotesChecker {
    account_id: AccountId,
    notes: Vec<InputNote>,
}

impl NotesChecker {
    /// Returns a new instance of the [NotesChecker].
    pub fn new(account_id: AccountId, notes: Vec<InputNote>) -> Self {
        NotesChecker { account_id, notes }
    }

    /// Checks whether the provided input notes could be consumed by the provided account.
    ///
    /// This check consists of two main steps:
    /// - Statically check the notes: if all notes are either `P2ID` or `P2IDR` notes with correct
    ///   inputs, return `NoteAccountExecution::Success`.
    /// - Execute the transaction:
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

        let mut successful_notes = vec![];
        for note in self.notes.iter() {
            if let Some(well_known_note) = WellKnownNote::from_note(note.note()) {
                if let WellKnownNote::SWAP = well_known_note {
                    // if we encountered a SWAP note, then we have to execute the transaction
                    // anyway, so we can stop checking
                    break;
                } else if let NoteAccountCompatibility::No =
                    well_known_note.check_note_inputs(note.note(), self.account_id)
                {
                    // return a `Failure` with the vector of successfully checked `P2ID` and
                    // `P2IDR` notes if the check failed
                    return Ok(NoteAccountExecution::Failure {
                        failed_note_id: note.id(),
                        successful_notes,
                        error: None,
                    });
                } else {
                    // put the successfully checked `P2ID` or `P2IDR` note to the vector
                    successful_notes.push(note.id());
                }
            } else {
                // if we encountered not a well known note, then we have to execute the transaction
                // anyway, so we can stop checking
                break;
            }
        }

        // if all checked notes turned out to be either `P2ID` or `P2IDR` notes and all of them
        // passed, then we could safely return the `Success`
        if successful_notes.len() == self.notes.len() {
            return Ok(NoteAccountExecution::Success);
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
