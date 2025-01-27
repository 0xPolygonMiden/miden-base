use miden_objects::{account::AccountId, note::NoteId, BatchAccountUpdateError};
use thiserror::Error;
use vm_processor::Digest;

/// Error encountered while building a batch.
#[derive(Debug, Error)]
pub enum BatchError {
    #[error("duplicated unauthenticated transaction input note ID in the batch: {0}")]
    DuplicateUnauthenticatedNote(NoteId),

    #[error("duplicated transaction output note ID in the batch: {0}")]
    DuplicateOutputNote(NoteId),

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
