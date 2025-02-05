use alloc::{collections::BTreeMap, vec::Vec};

use crate::{
    account::AccountId,
    batch::{BatchAccountUpdate, BatchId, BatchNoteTree},
    block::BlockNumber,
    note::Nullifier,
    transaction::{InputNoteCommitment, InputNotes, OutputNote},
};

/// A transaction batch with an execution proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenBatch {
    id: BatchId,
    account_updates: BTreeMap<AccountId, BatchAccountUpdate>,
    input_notes: InputNotes<InputNoteCommitment>,
    output_notes_smt: BatchNoteTree,
    output_notes: Vec<OutputNote>,
    batch_expiration_block_num: BlockNumber,
}

impl ProvenBatch {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`ProvenBatch`] from the provided parts.
    pub fn new(
        id: BatchId,
        account_updates: BTreeMap<AccountId, BatchAccountUpdate>,
        input_notes: InputNotes<InputNoteCommitment>,
        output_notes_smt: BatchNoteTree,
        output_notes: Vec<OutputNote>,
        batch_expiration_block_num: BlockNumber,
    ) -> Self {
        Self {
            id,
            account_updates,
            input_notes,
            output_notes_smt,
            output_notes,
            batch_expiration_block_num,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// The ID of this batch. See [`BatchId`] for details on how it is computed.
    pub fn id(&self) -> BatchId {
        self.id
    }

    /// Returns the block number at which the batch will expire.
    pub fn batch_expiration_block_num(&self) -> BlockNumber {
        self.batch_expiration_block_num
    }

    /// Returns the map of account IDs mapped to their [`BatchAccountUpdate`]s.
    ///
    /// If an account was updated by multiple transactions, the [`BatchAccountUpdate`] is the result
    /// of merging the individual updates.
    ///
    /// For example, suppose an account's state before this batch is `A` and the batch contains two
    /// transactions that updated it. Applying the first transaction results in intermediate state
    /// `B`, and applying the second one results in state `C`. Then the returned update represents
    /// the state transition from `A` to `C`.
    pub fn account_updates(&self) -> &BTreeMap<AccountId, BatchAccountUpdate> {
        &self.account_updates
    }

    /// Takes the map of account IDs mapped to their [`BatchAccountUpdate`]s from the proven batch.
    ///
    /// This has the semantics of [`core::mem::take`], i.e. the account updates are set to an empty
    /// `BTreeMap` after this operation.
    pub fn take_account_updates(&mut self) -> BTreeMap<AccountId, BatchAccountUpdate> {
        core::mem::take(&mut self.account_updates)
    }

    /// Returns the [`InputNotes`] of this batch.
    pub fn input_notes(&self) -> &InputNotes<InputNoteCommitment> {
        &self.input_notes
    }

    /// Returns an iterator over the nullifiers produced in this batch.
    pub fn produced_nullifiers(&self) -> impl Iterator<Item = Nullifier> + use<'_> {
        self.input_notes.iter().map(InputNoteCommitment::nullifier)
    }

    /// Returns the output notes of the batch.
    ///
    /// This is the aggregation of all output notes by the transactions in the batch, except the
    /// ones that were consumed within the batch itself.
    pub fn output_notes(&self) -> &[OutputNote] {
        &self.output_notes
    }

    /// Returns the [`BatchNoteTree`] representing the output notes of the batch.
    pub fn output_notes_tree(&self) -> &BatchNoteTree {
        &self.output_notes_smt
    }
}
