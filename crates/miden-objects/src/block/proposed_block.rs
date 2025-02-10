use alloc::vec::Vec;
use std::collections::{BTreeMap, BTreeSet};

use vm_processor::Digest;

use crate::{
    account::{delta::AccountUpdateDetails, AccountId},
    batch::{BatchAccountUpdate, BatchId, InputOutputNoteTracker, ProvenBatch},
    block::{
        block_inputs::BlockInputs, AccountUpdateWitness, AccountWitness, BlockHeader,
        BlockNoteTree, BlockNumber, NullifierWitness, PartialNullifierTree,
    },
    errors::ProposedBlockError,
    note::{NoteId, Nullifier},
    transaction::{ChainMmr, InputNoteCommitment, OutputNote, TransactionId},
    MAX_BATCHES_PER_BLOCK,
};

type UpdatedAccounts = Vec<(AccountId, AccountUpdateWitness)>;

// BLOCK WITNESS
// =================================================================================================

/// Provides inputs to the `BlockKernel` so that it can generate the new header.
#[derive(Debug, Clone)]
pub struct ProposedBlock {
    batches: Vec<ProvenBatch>,
    updated_accounts: Vec<(AccountId, AccountUpdateWitness)>,
    block_note_tree: BlockNoteTree,
    created_nullifiers: BTreeMap<Nullifier, NullifierWitness>,
    chain_mmr: ChainMmr,
    prev_block_header: BlockHeader,
}

impl ProposedBlock {
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// ## Block
    ///
    /// - TODO
    ///
    /// ## Accounts
    /// - an [`AccountWitness`] is missing for an account updated by a batch.
    /// - any two batches update the same account from the same state. For example, if batch 1
    ///   updates some account from state A to B and batch 2 updates it from A to F, then those
    ///   batches conflict as they both start from the same initial state but produce a fork in the
    ///   account's state.
    /// - account updates from different batches cannot be brought in a contiguous order. For
    ///   example, if a batch 1 updates an account from state A to C, and a batch 2 updates it from
    ///   D to F, then the state transition from C to D is missing. Note that this does not mean,
    ///   that batches must be provided in an order where account updates chain together in the
    ///   order of the batches, which would generally be an impossible requirement to fulfill.
    /// - account updates cannot be merged, i.e. if [`AccountUpdateDetails::merge`] fails on the
    ///   updates from two batches.
    pub fn new(
        mut block_inputs: BlockInputs,
        mut batches: Vec<ProvenBatch>,
    ) -> Result<Self, ProposedBlockError> {
        // Check for empty or duplicate batches.
        // --------------------------------------------------------------------------------------------

        if batches.is_empty() {
            return Err(ProposedBlockError::EmptyBlock);
        }

        if batches.len() > MAX_BATCHES_PER_BLOCK {
            return Err(ProposedBlockError::TooManyBatches);
        }

        check_duplicate_batches(&batches)?;

        // Check for consistency between the chain MMR and the referenced previous block.
        // --------------------------------------------------------------------------------------------

        check_reference_block_chain_mmr_consistency(
            block_inputs.chain_mmr(),
            block_inputs.prev_block_header(),
        )?;

        // Check every block referenced by a batch is in the chain MMR.
        // --------------------------------------------------------------------------------------------

        check_batch_reference_blocks(
            block_inputs.chain_mmr(),
            block_inputs.prev_block_header(),
            &batches,
        )?;

        // Check for duplicates in the input and output notes and compute the input and output notes
        // of the block by erasing notes that are created and consumed within this block as well as
        // authenticating unauthenticated notes.
        // --------------------------------------------------------------------------------------------

        let (block_input_notes, block_output_notes) = InputOutputNoteTracker::from_batches(
            batches.iter(),
            block_inputs.unauthenticated_note_proofs(),
            block_inputs.chain_mmr(),
        )?;

        // All unauthenticated notes must be erased or authenticated by now.
        if let Some(nullifier) = block_input_notes
            .iter()
            .find_map(|note| (!note.is_authenticated()).then_some(note.nullifier()))
        {
            return Err(ProposedBlockError::UnauthenticatedNoteConsumed { nullifier });
        }

        // Check for nullifiers proofs and unspent nullifiers.
        // --------------------------------------------------------------------------------------------

        // Check against computed block_input_notes which also contain unauthenticated notes that
        // have been authenticated.
        check_nullifiers(
            &block_inputs,
            block_input_notes.iter().map(InputNoteCommitment::nullifier),
        )?;

        // Aggregate account updates across batches.
        // --------------------------------------------------------------------------------------------

        let account_witnesses = aggregate_account_updates(&mut block_inputs, &mut batches)?;

        // Compute the block note tree from the individual batch note trees.
        // --------------------------------------------------------------------------------------------

        let block_note_tree = compute_block_note_tree(&batches, &block_output_notes);

        // Build proposed blocks from parts.
        // --------------------------------------------------------------------------------------------
        let (prev_block_header, chain_mmr, _, nullifiers, _) = block_inputs.into_parts();

        Ok(Self {
            batches,
            updated_accounts: account_witnesses,
            block_note_tree,
            created_nullifiers: nullifiers,
            chain_mmr,
            prev_block_header,
        })
    }

    /// Returns an iterator over all transactions which affected accounts in the block with
    /// corresponding account IDs.
    pub fn affected_accounts(&self) -> impl Iterator<Item = (TransactionId, AccountId)> + '_ {
        self.updated_accounts.iter().flat_map(|(account_id, update)| {
            update.transactions().iter().map(move |tx_id| (*tx_id, *account_id))
        })
    }

    /// Returns the block number of this proposed block.
    pub fn block_num(&self) -> BlockNumber {
        self.chain_mmr().chain_length()
    }

    pub fn batches(&self) -> &[ProvenBatch] {
        &self.batches
    }

    pub fn batches_mut(&mut self) -> &mut [ProvenBatch] {
        &mut self.batches
    }

    /// Returns the map of nullifiers to their proofs from the proposed block.
    pub fn nullifiers(&self) -> &BTreeMap<Nullifier, NullifierWitness> {
        &self.created_nullifiers
    }

    pub fn prev_block_header(&self) -> &BlockHeader {
        &self.prev_block_header
    }

    pub fn chain_mmr(&self) -> &ChainMmr {
        &self.chain_mmr
    }

    pub fn updated_accounts(&self) -> &[(AccountId, AccountUpdateWitness)] {
        &self.updated_accounts
    }

    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        Vec<ProvenBatch>,
        Vec<(AccountId, AccountUpdateWitness)>,
        BlockNoteTree,
        BTreeMap<Nullifier, NullifierWitness>,
        ChainMmr,
        BlockHeader,
    ) {
        (
            self.batches,
            self.updated_accounts,
            self.block_note_tree,
            self.created_nullifiers,
            self.chain_mmr,
            self.prev_block_header,
        )
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn check_duplicate_batches(batches: &[ProvenBatch]) -> Result<(), ProposedBlockError> {
    let mut input_note_set = BTreeSet::new();

    for batch in batches {
        if !input_note_set.insert(batch.id()) {
            return Err(ProposedBlockError::DuplicateBatch { batch_id: batch.id() });
        }
    }

    Ok(())
}

/// Check that each nullifier in the block has a proof provided and that the nullifier is
/// unspent. The proofs are required to update the nullifier tree.
fn check_nullifiers(
    block_inputs: &BlockInputs,
    batch_nullifiers: impl Iterator<Item = Nullifier>,
) -> Result<(), ProposedBlockError> {
    for batch_nullifier in batch_nullifiers {
        match block_inputs.nullifiers().get(&batch_nullifier) {
            Some(witness) => {
                let (_, nullifier_value) = witness
                    .proof()
                    .leaf()
                    .entries()
                    .iter()
                    .find(|(key, _)| *key == batch_nullifier.inner())
                    .ok_or(ProposedBlockError::NullifierProofMissing(batch_nullifier))?;

                if *nullifier_value != PartialNullifierTree::UNSPENT_NULLIFIER_VALUE {
                    return Err(ProposedBlockError::NullifierSpent(batch_nullifier));
                }
            },
            None => return Err(ProposedBlockError::NullifierProofMissing(batch_nullifier)),
        }
    }

    Ok(())
}

fn check_reference_block_chain_mmr_consistency(
    chain_mmr: &ChainMmr,
    prev_block_header: &BlockHeader,
) -> Result<(), ProposedBlockError> {
    // Make sure that the current chain MMR has blocks up to prev_block_header - 1, i.e. its
    // chain length is equal to the block number of the previous block header.
    if chain_mmr.chain_length() != prev_block_header.block_num() {
        return Err(ProposedBlockError::ChainLengthNotEqualToPreviousBlockNumber {
            chain_length: chain_mmr.chain_length(),
            prev_block_num: prev_block_header.block_num(),
        });
    }

    let chain_root = chain_mmr.peaks().hash_peaks();
    if chain_root != prev_block_header.chain_root() {
        return Err(ProposedBlockError::ChainRootNotEqualToPreviousBlockChainRoot {
            chain_root,
            prev_block_chain_root: prev_block_header.chain_root(),
            prev_block_num: prev_block_header.block_num(),
        });
    }

    Ok(())
}

/// Check that each block referenced by a batch in the block has an entry in the chain MMR,
/// except if the referenced block is the same as the previous block, referenced by the block.
fn check_batch_reference_blocks(
    chain_mmr: &ChainMmr,
    prev_block_header: &BlockHeader,
    batches: &[ProvenBatch],
) -> Result<(), ProposedBlockError> {
    for batch in batches {
        let batch_reference_block_num = batch.reference_block_num();
        if batch_reference_block_num != prev_block_header.block_num()
            && !chain_mmr.contains_block(batch.reference_block_num())
        {
            return Err(ProposedBlockError::BatchReferenceBlockMissingFromChain {
                reference_block_num: batch.reference_block_num(),
                batch_id: batch.id(),
            });
        }
    }

    Ok(())
}

/// Computes the [`BlockNoteTree`] from the note trees of the batches in the block.
///
/// This takes the batch note tree of a batch and removes any note that was erased (i.e. consumed by
/// another batch in the block), as dictated by the `block_output_notes` map.
///
/// Then inserts the batch note tree as a subtree into the larger block note tree.
fn compute_block_note_tree(
    batches: &[ProvenBatch],
    block_output_notes: &BTreeMap<NoteId, (BatchId, OutputNote)>,
) -> BlockNoteTree {
    let mut block_note_tree = BlockNoteTree::empty();

    for (batch_idx, batch) in batches.iter().enumerate() {
        let mut batch_output_notes_tree = batch.output_notes_tree().clone();

        for (note_tree_idx, original_output_note) in batch.output_notes().iter().enumerate() {
            // Note that because we disallow duplicate output notes, if this map contains a note id,
            // then we can be certain it was created by this batch and should stay in the tree.
            if !block_output_notes.contains_key(&original_output_note.id()) {
                let index = u64::try_from(note_tree_idx).expect(
                  "the number of output notes should be less than MAX_OUTPUT_NOTES_PER_BATCH and thus fit into a u64",
              );
                batch_output_notes_tree
                    .remove(index)
                    .expect("the index should be less than MAX_OUTPUT_NOTES_PER_BATCH");
            }
        }

        let batch_idx = u64::try_from(batch_idx)
            .expect("the index should be less than MAX_BATCHES_PER_BLOCK and thus fit into a u64");
        block_note_tree
            .insert_batch_note_subtree(batch_idx, batch_output_notes_tree)
            .expect("the batch note tree depth should not exceed the block note tree depth and the index should be less than MAX_BATCHES_PER_BLOCK");
    }

    block_note_tree
}

/// Aggregate all updates for the same account and store each update indexed by its initial
/// state commitment so we can easily retrieve them later.
/// This lets us chronologically order the updates per account across batches.
fn aggregate_account_updates(
    block_inputs: &mut BlockInputs,
    batches: &mut [ProvenBatch],
) -> Result<UpdatedAccounts, ProposedBlockError> {
    let mut update_aggregator = AccountUpdateAggregator::new();

    for batch in batches {
        for (account_id, update) in batch.take_account_updates() {
            update_aggregator.insert_update(account_id, batch.id(), update)?;
        }
    }

    update_aggregator.aggregate_all(block_inputs)
}

struct AccountUpdateAggregator {
    /// The map from each account to the map of each of its updates, where the digest is the state
    /// commitment from which the contained update starts.
    /// An invariant of this field is that if the outer map has an entry for some account, the
    /// inner update map is guaranteed to not be empty as well.
    updates: BTreeMap<AccountId, BTreeMap<Digest, (BatchAccountUpdate, BatchId)>>,
}

impl AccountUpdateAggregator {
    fn new() -> Self {
        Self { updates: BTreeMap::new() }
    }

    /// Inserts the update from one batch for a specific account into the map of updates.
    fn insert_update(
        &mut self,
        account_id: AccountId,
        batch_id: BatchId,
        update: BatchAccountUpdate,
    ) -> Result<(), ProposedBlockError> {
        if let Some((conflicting_update, conflicting_batch_id)) = self
            .updates
            .entry(account_id)
            .or_default()
            .insert(update.initial_state_commitment(), (update, batch_id))
        {
            return Err(ProposedBlockError::ConflictingBatchesUpdateSameAccount {
                account_id,
                initial_state_commitment: conflicting_update.initial_state_commitment(),
                first_batch_id: conflicting_batch_id,
                second_batch_id: batch_id,
            });
        }

        Ok(())
    }

    /// Consumes self and aggregates the account updates from all contained accounts.
    fn aggregate_all(
        self,
        block_inputs: &mut BlockInputs,
    ) -> Result<UpdatedAccounts, ProposedBlockError> {
        let mut account_witnesses = Vec::with_capacity(self.updates.len());

        for (account_id, updates_map) in self.updates {
            let witness = block_inputs
                .accounts_mut()
                .remove(&account_id)
                .ok_or(ProposedBlockError::MissingAccountWitness(account_id))?;

            let account_update_witness = Self::aggregate_account(account_id, witness, updates_map)?;

            account_witnesses.push((account_id, account_update_witness));
        }

        Ok(account_witnesses)
    }

    /// Build the update for a single account from the provided map of updates, where each entry is
    /// the state from which the update starts. This chains updates for this account together in a
    /// chronological order using the state commitments to link them.
    fn aggregate_account(
        account_id: AccountId,
        witness: AccountWitness,
        mut updates: BTreeMap<Digest, (BatchAccountUpdate, BatchId)>,
    ) -> Result<AccountUpdateWitness, ProposedBlockError> {
        let (initial_state_commitment, initial_state_proof) = witness.into_parts();
        let mut details: Option<AccountUpdateDetails> = None;

        let mut transactions = Vec::new();
        let mut current_commitment = initial_state_commitment;
        while !updates.is_empty() {
            let (update, _) = updates.remove(&current_commitment).ok_or_else(|| {
                ProposedBlockError::InconsistentAccountStateTransition(
                    account_id,
                    current_commitment,
                    updates.keys().copied().collect(),
                )
            })?;

            current_commitment = update.final_state_commitment();
            let (update_transactions, update_details) = update.into_parts();
            transactions.extend(update_transactions);

            details = Some(match details {
                None => update_details,
                Some(details) => details.merge(update_details).map_err(|source| {
                    ProposedBlockError::AccountUpdateError { account_id, source }
                })?,
            });
        }

        Ok(AccountUpdateWitness::new(
            initial_state_commitment,
            current_commitment,
            initial_state_proof,
            details.expect("details should be Some as updates is guaranteed to not be empty"),
            transactions,
        ))
    }
}
