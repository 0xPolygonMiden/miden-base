use miden_objects::{
    account::AccountId, note::NoteId, transaction::TransactionId, BatchAccountUpdateError,
};
use thiserror::Error;
use vm_processor::Digest;

/// Error encountered while building a batch.
#[derive(Debug, Error)]
pub enum BatchError {
    #[error("duplicated unauthenticated transaction input note ID in the batch: {0}")]
    DuplicateUnauthenticatedNote(NoteId),

    #[error("transaction {second_transaction_id} outputs a note with id {note_id} that is also produced by the previous transaction {first_transaction_id} in the batch")]
    DuplicateOutputNote {
        note_id: NoteId,
        first_transaction_id: TransactionId,
        second_transaction_id: TransactionId,
    },

    #[error("note hashes mismatch for note {id}: (input: {input_hash}, output: {output_hash})")]
    NoteHashesMismatch {
        id: NoteId,
        input_hash: Digest,
        output_hash: Digest,
    },

    #[error("failed to merge transaction delta into account {account_id}")]
    AccountUpdateError {
        account_id: AccountId,
        source: BatchAccountUpdateError,
    },
}
