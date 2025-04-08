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

use super::{ExecutionCheckResult, TransactionExecutor};
use crate::TransactionExecutorError;

pub struct NotesChecker {
    account_id: AccountId,
    notes: Vec<InputNote>,
}

impl NotesChecker {
    pub fn new(account_id: AccountId, notes: Vec<InputNote>) -> Self {
        NotesChecker { account_id, notes }
    }

    /// Checks whether there are "well known" notes (`P2ID`, `P2IDR` and `SWAP`) in the list of the
    /// provided input notes. If so, assert that the note inputs are correct.
    ///
    /// Returns [NoteAccountCompatibility::No] if at least one note has incorrect inputs.
    pub fn check_note_inputs(&self) -> (NoteAccountCompatibility, Option<NoteId>) {
        for note in self.notes.iter() {
            if let Some(well_known_note) = WellKnownNote::from_note(note.note()) {
                if let NoteAccountCompatibility::No =
                    well_known_note.check_note_inputs(note.note(), self.account_id)
                {
                    return (NoteAccountCompatibility::No, Some(note.id()));
                }
            }
        }

        (NoteAccountCompatibility::Maybe, None)
    }

    /// Checks whether the provided input notes could be consumed by the provided account.
    ///
    /// This check consists of two main steps:
    /// - Check whether there are "well known" notes (`P2ID`, `P2IDR` and `SWAP`) in the list of the
    ///   provided input notes. If so, assert that the note inputs are correct.
    /// - Execute the transaction with specified notes.
    ///   - Returns [`ExecutionCheckResult::Success`] if the execution was successful.
    ///   - Returns [`ExecutionCheckResult::Failure`] if some note returned an error. The tuple
    ///     associated with `Failure` variant contains the ID of the failing note and a vector of
    ///     IDs of the notes, which were successfully executed.
    #[maybe_async]
    pub fn check_notes_consumability(
        &self,
        tx_executor: &TransactionExecutor,
        block_ref: BlockNumber,
        tx_args: TransactionArgs,
    ) -> Result<ExecutionCheckResult, TransactionExecutorError> {
        // Check input notes
        // ----------------------------------------------------------------------------------------

        let inputs_check_result = self.check_note_inputs();
        if let (NoteAccountCompatibility::No, failing_note_id) = inputs_check_result {
            return Ok(ExecutionCheckResult::Failure((
                failing_note_id.expect("tuple with incompatible note should contain its ID"),
                vec![],
            )));
        }

        // Execute transaction
        // ----------------------------------------------------------------------------------------
        let note_ids = self.notes.iter().map(|note| note.id()).collect::<Vec<NoteId>>();
        maybe_await!(tx_executor.notes_execution_progress_checker(
            self.account_id,
            block_ref,
            &note_ids,
            tx_args
        ))
    }
}
