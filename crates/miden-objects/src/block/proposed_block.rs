use alloc::vec::Vec;
use std::collections::{BTreeMap, BTreeSet};

use vm_processor::Digest;

use crate::{
    account::{delta::AccountUpdateDetails, AccountId},
    batch::{BatchAccountUpdate, BatchId, InputOutputNoteTracker, ProvenBatch},
    block::{
        block_inputs::BlockInputs, BlockAccountUpdate, BlockHeader, BlockNoteIndex, BlockNoteTree,
        BlockNumber, NullifierWitness, PartialNullifierTree,
    },
    crypto::merkle::MerklePath,
    errors::ProposedBlockError,
    note::Nullifier,
    transaction::{ChainMmr, InputNoteCommitment, TransactionId},
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
    pub fn new(
        mut block_inputs: BlockInputs,
        mut batches: Vec<ProvenBatch>,
    ) -> Result<(Self, Vec<BlockAccountUpdate>), ProposedBlockError> {
        // Check for empty or duplicate batches.
        // --------------------------------------------------------------------------------------------

        if batches.is_empty() {
            return Err(ProposedBlockError::EmptyBlock);
        }

        if batches.len() > MAX_BATCHES_PER_BLOCK {
            return Err(ProposedBlockError::TooManyBatches);
        }

        check_duplicate_batches(&batches)?;

        // Check for consistency in chain MMR and referenced prev block.
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
        // making sure that authenticating unauthenticated notes.
        // --------------------------------------------------------------------------------------------

        let (block_input_notes, mut block_output_notes) = InputOutputNoteTracker::from_batches(
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

        // Create the BlockNoteTree from the block output notes, where created unauthenticated notes
        // that are consumed by another batch are erased.
        // This ensures that the batch index in the block note tree matches the index of the batch
        // in batches.
        let batch_output_notes_iterator = batches
        .iter()
        .enumerate()
        // Filter out batches that do not have an entry in block output notes, which could happen for
        // batches that don't produce output notes.
        .filter_map(|(batch_idx, batch)| {
            block_output_notes.remove(&batch.id()).map(|output_notes| (batch_idx, output_notes))
        })
        .flat_map(|(batch_idx, output_notes)| {
            output_notes.into_iter().enumerate().map(move |(note_idx_in_batch, note)| {
                // SAFETY: This is fine because:
                // - we check for MAX_BATCHES_PER_BLOCK in this function,
                // - and max output notes per batch is enforced by the `ProposedBatch`.
                let block_note_idx = BlockNoteIndex::new(batch_idx, note_idx_in_batch).expect("we should not exceed the max output notes per batch or the number of batches per block");
                (block_note_idx, note.id(), *note.metadata())
            })
        });

        let block_note_tree =
            BlockNoteTree::with_entries(batch_output_notes_iterator).expect("TODO: error");

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

        let (account_witnesses, block_updates) =
            aggregate_account_updates(&mut block_inputs, &mut batches)?;

        // Build proposed blocks from parts.
        // --------------------------------------------------------------------------------------------
        let (prev_block_header, chain_mmr, _, nullifiers, _) = block_inputs.into_parts();

        Ok((
            Self {
                batches,
                updated_accounts: account_witnesses,
                block_note_tree,
                created_nullifiers: nullifiers,
                chain_mmr,
                prev_block_header,
            },
            block_updates,
        ))
    }

    /// Returns an iterator over all transactions which affected accounts in the block with
    /// corresponding account IDs.
    pub fn affected_accounts(&self) -> impl Iterator<Item = (TransactionId, AccountId)> + '_ {
        self.updated_accounts.iter().flat_map(|(account_id, update)| {
            update.transactions.iter().map(move |tx_id| (*tx_id, *account_id))
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

fn aggregate_account_updates(
    block_inputs: &mut BlockInputs,
    batches: &mut [ProvenBatch],
) -> Result<(UpdatedAccounts, Vec<BlockAccountUpdate>), ProposedBlockError> {
    // TODO: A HashMap would be much more efficient here as we don't need the order. We also
    // rebalance the tree when removing the updates which is also unnecessary.

    // Aggregate all updates for the same account and store each update indexed by its initial
    // state commitment so we can easily retrieve them later.
    // This lets us chronologically order the updates per account across batches.
    let mut updated_accounts =
        BTreeMap::<AccountId, BTreeMap<Digest, (BatchAccountUpdate, BatchId)>>::new();

    for batch in batches {
        for (account_id, update) in batch.take_account_updates() {
            if let Some((conflicting_update, conflicting_batch_id)) = updated_accounts
                .entry(account_id)
                .or_default()
                .insert(update.initial_state_commitment(), (update, batch.id()))
            {
                return Err(ProposedBlockError::ConflictingBatchesUpdateSameAccount {
                    account_id,
                    initial_state_commitment: conflicting_update.initial_state_commitment(),
                    first_batch_id: conflicting_batch_id,
                    second_batch_id: batch.id(),
                });
            }
        }
    }

    // Build account witnesses.
    let mut account_witnesses = Vec::with_capacity(updated_accounts.len());
    let mut block_updates = Vec::with_capacity(updated_accounts.len());

    for (account_id, mut updates) in updated_accounts {
        let (initial_state_commitment, proof) = block_inputs
            .accounts_mut()
            .remove(&account_id)
            .map(|witness| witness.into_parts())
            .ok_or(ProposedBlockError::MissingAccountWitness(account_id))?;

        let mut details: Option<AccountUpdateDetails> = None;

        // Chronologically chain updates for this account together using the state hashes to
        // link them.
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

        account_witnesses.push((
            account_id,
            AccountUpdateWitness {
                initial_state_commitment,
                final_state_commitment: current_commitment,
                initial_state_proof: proof,
                transactions: core::mem::take(&mut transactions),
            },
        ));

        block_updates.push(BlockAccountUpdate::new(
            account_id,
            current_commitment,
            details.expect("Must be some by now"),
            transactions,
        ));
    }

    Ok((account_witnesses, block_updates))
}

/// TODO
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountUpdateWitness {
    initial_state_commitment: Digest,
    final_state_commitment: Digest,
    initial_state_proof: MerklePath,
    transactions: Vec<TransactionId>,
}

impl AccountUpdateWitness {
    /// Constructs a new [`AccountUpdateWitness`] from the provided parts.
    pub fn new(
        initial_state_commitment: Digest,
        final_state_commitment: Digest,
        initial_state_proof: MerklePath,
        transactions: Vec<TransactionId>,
    ) -> Self {
        Self {
            initial_state_commitment,
            final_state_commitment,
            initial_state_proof,
            transactions,
        }
    }

    /// Returns the initial state commitment of the account.
    pub fn initial_state_commitment(&self) -> Digest {
        self.initial_state_commitment
    }

    /// Returns the final state commitment of the account.
    pub fn final_state_commitment(&self) -> Digest {
        self.final_state_commitment
    }

    /// Returns a reference to the initial state proof of the account.
    pub fn initial_state_proof(&self) -> &MerklePath {
        &self.initial_state_proof
    }

    /// Returns a mutable reference to the initial state proof of the account.
    pub fn initial_state_proof_mut(&mut self) -> &mut MerklePath {
        &mut self.initial_state_proof
    }

    /// Returns the transactions that affected the account.
    pub fn transactions(&self) -> &[TransactionId] {
        &self.transactions
    }

    /// Consumes self and returns its parts.
    pub fn into_parts(self) -> (Digest, Digest, MerklePath, Vec<TransactionId>) {
        (
            self.initial_state_commitment,
            self.final_state_commitment,
            self.initial_state_proof,
            self.transactions,
        )
    }
}
