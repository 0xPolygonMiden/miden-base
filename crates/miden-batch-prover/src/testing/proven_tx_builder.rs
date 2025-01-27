use alloc::vec::Vec;

use anyhow::Context;
use miden_objects::{
    account::AccountId,
    block::BlockNumber,
    note::{Note, Nullifier},
    transaction::{InputNote, OutputNote, ProvenTransaction, ProvenTransactionBuilder},
    vm::ExecutionProof,
};
use vm_processor::Digest;
use winterfell::Proof;

/// A builder to build mocked [`ProvenTransaction`]s.
pub struct MockProvenTxBuilder {
    account_id: AccountId,
    initial_account_commitment: Digest,
    final_account_commitment: Digest,
    expiration_block_num: BlockNumber,
    output_notes: Option<Vec<OutputNote>>,
    input_notes: Option<Vec<InputNote>>,
    nullifiers: Option<Vec<Nullifier>>,
}

impl MockProvenTxBuilder {
    /// Creates a new builder for a transaction executed against the given account with its initial
    /// and final state commitment.
    pub fn with_account(
        account_id: AccountId,
        initial_account_commitment: Digest,
        final_account_commitment: Digest,
    ) -> Self {
        Self {
            account_id,
            initial_account_commitment,
            final_account_commitment,
            expiration_block_num: BlockNumber::from(u32::MAX),
            output_notes: None,
            input_notes: None,
            nullifiers: None,
        }
    }

    /// Adds unauthenticated notes to the transaction.
    #[must_use]
    pub fn unauthenticated_notes(mut self, notes: Vec<Note>) -> Self {
        self.input_notes = Some(notes.into_iter().map(InputNote::unauthenticated).collect());

        self
    }

    /// Adds nullifiers to the transaction's input note commitment.
    #[must_use]
    pub fn nullifiers(mut self, nullifiers: Vec<Nullifier>) -> Self {
        self.nullifiers = Some(nullifiers);

        self
    }

    /// Sets the transaction's expiration block number.
    #[must_use]
    pub fn expiration_block_num(mut self, expiration_block_num: BlockNumber) -> Self {
        self.expiration_block_num = expiration_block_num;

        self
    }

    /// Adds notes to the transaction's output notes.
    #[must_use]
    pub fn output_notes(mut self, notes: Vec<OutputNote>) -> Self {
        self.output_notes = Some(notes);

        self
    }

    /// Builds the [`ProvenTransaction`] and returns potential errors.
    pub fn build(self) -> anyhow::Result<ProvenTransaction> {
        ProvenTransactionBuilder::new(
            self.account_id,
            self.initial_account_commitment,
            self.final_account_commitment,
            Digest::default(),
            self.expiration_block_num,
            ExecutionProof::new(Proof::new_dummy(), Default::default()),
        )
        .add_input_notes(self.input_notes.unwrap_or_default())
        .add_input_notes(self.nullifiers.unwrap_or_default())
        .add_output_notes(self.output_notes.unwrap_or_default())
        .build()
        .context("failed to build proven transaction")
    }
}
