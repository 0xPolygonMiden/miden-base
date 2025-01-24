use alloc::vec::Vec;

use miden_objects::{
    account::AccountUpdate,
    batch::{BatchId, BatchNoteTree},
    transaction::{InputNoteCommitment, OutputNotes},
};

// TODO: Document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenBatch {
    id: BatchId,
    account_updates: Vec<AccountUpdate>,
    input_notes: Vec<InputNoteCommitment>,
    output_notes_smt: BatchNoteTree,
    output_notes: OutputNotes,
}

impl ProvenBatch {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`ProvenBatch`] from the provided parts.
    pub fn new(
        id: BatchId,
        account_updates: Vec<AccountUpdate>,
        input_notes: Vec<InputNoteCommitment>,
        output_notes_smt: BatchNoteTree,
        output_notes: OutputNotes,
    ) -> Self {
        Self {
            id,
            account_updates,
            input_notes,
            output_notes_smt,
            output_notes,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// The ID of this batch. See [`BatchId`] for details on how it is computed.
    pub fn id(&self) -> BatchId {
        self.id
    }

    /// Returns a slice of [`AccountUpdate`]s - exactly one for each account updated in the batch.
    ///
    /// If an account was updated by multiple transactions, the returned [`AccountUpdate`] is the
    /// result of merging the individual updates.
    ///
    /// For example, suppose an account's state before this batch is `A` and the batch contains two
    /// transactions that updated it. Applying the first transaction results in intermediate state
    /// `B`, and applying the second one results in state `C`. Then the returned update represents
    /// the state transition from `A` to `C`.
    pub fn account_updates(&self) -> &[AccountUpdate] {
        &self.account_updates
    }

    /// Returns the output notes of the batch.
    ///
    /// This is the aggregation of all output notes by the contained transactions, except the ones
    /// that were consumed within the batch itself.
    pub fn output_notes(&self) -> &OutputNotes {
        &self.output_notes
    }

    /// Returns the [`BatchNoteTree`] representing the output notes of the batch.
    ///
    /// TODO: More docs?
    pub fn output_notes_tree(&self) -> &BatchNoteTree {
        &self.output_notes_smt
    }
}
