use miden_objects::{
    account::AccountId,
    block::BlockNumber,
    note::{NoteId, Nullifier},
    transaction::TransactionId,
    BatchAccountUpdateError,
};
use thiserror::Error;
use vm_processor::Digest;

/// Errors encountered while building a transaction batch.
#[derive(Debug, Error)]
pub enum BatchError {
    #[error("transaction {second_transaction_id} consumes the note with nullifier {note_nullifier} that is also consumed by another transaction {first_transaction_id} in the batch")]
    DuplicateInputNote {
        note_nullifier: Nullifier,
        first_transaction_id: TransactionId,
        second_transaction_id: TransactionId,
    },

    #[error("transaction {second_transaction_id} creates the note with id {note_id} that is also created by another transaction {first_transaction_id} in the batch")]
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

    #[error("unauthenticated input note with id {note_id} for which an inclusion proof was provided was not created in block {block_num}")]
    UnauthenticatedNoteAuthenticationFailed { note_id: NoteId, block_num: BlockNumber },
}
