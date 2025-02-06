use alloc::vec::Vec;
use std::collections::{BTreeMap, BTreeSet};

use vm_processor::Digest;

use crate::{
    account::{delta::AccountUpdateDetails, AccountId},
    batch::{BatchAccountUpdate, BatchId, ProvenBatch},
    block::{
        block_inputs::BlockInputs, BlockAccountUpdate, BlockHeader, BlockNumber,
        PartialNullifierTree,
    },
    crypto::merkle::{MerklePath, SmtProof},
    errors::ProposedBlockError,
    note::Nullifier,
    transaction::{ChainMmr, TransactionId},
    MAX_BATCHES_PER_BLOCK,
};

// BLOCK WITNESS
// =================================================================================================

/// Provides inputs to the `BlockKernel` so that it can generate the new header.
#[derive(Debug, PartialEq)]
pub struct ProposedBlock {
    batches: Vec<ProvenBatch>,
    updated_accounts: Vec<(AccountId, AccountUpdateWitness)>,
    /// Map from batch index to its output notes SMT root.
    ///
    /// There may be no entry for a given batch index if that batch did not create output notes.
    batch_created_notes_roots: BTreeMap<usize, Digest>,
    created_nullifiers: BTreeMap<Nullifier, SmtProof>,
    chain_mmr: ChainMmr,
    prev_block_header: BlockHeader,
}

impl ProposedBlock {
    pub fn new(
        mut block_inputs: BlockInputs,
        mut batches: Vec<ProvenBatch>,
    ) -> Result<(Self, Vec<BlockAccountUpdate>), ProposedBlockError> {
        // This limit should be enforced by the mempool.
        assert!(batches.len() <= MAX_BATCHES_PER_BLOCK);

        // Check for empty or duplicate batches.
        // --------------------------------------------------------------------------------------------

        if batches.is_empty() {
            return Err(ProposedBlockError::EmptyBlock);
        }

        check_duplicate_batches(&batches)?;

        // Check for duplicate input notes in batches.
        // --------------------------------------------------------------------------------------------

        check_duplicate_input_notes(&batches)?;

        // Check for duplicate output notes in batches.
        // --------------------------------------------------------------------------------------------

        check_duplicate_output_notes(&batches)?;

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

        // Check for nullifiers proofs and unspent nullifiers.
        // --------------------------------------------------------------------------------------------

        check_nullifiers(&block_inputs, &batches)?;

        // Collect output note SMT roots from batches.
        // --------------------------------------------------------------------------------------------

        let batch_created_notes_roots = batches
            .iter()
            .enumerate()
            .filter(|(_, batch)| !batch.output_notes().is_empty())
            .map(|(batch_index, batch)| (batch_index, batch.output_notes_tree().root()))
            .collect();

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
                batch_created_notes_roots,
                created_nullifiers: nullifiers,
                chain_mmr,
                prev_block_header,
            },
            block_updates,
        ))
    }

    /// Returns an iterator over all transactions which affected accounts in the block with
    /// corresponding account IDs.
    pub(super) fn transactions(&self) -> impl Iterator<Item = (TransactionId, AccountId)> + '_ {
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
    pub fn nullifiers(&self) -> &BTreeMap<Nullifier, SmtProof> {
        &self.created_nullifiers
    }

    pub fn prev_block_header(&self) -> &BlockHeader {
        &self.prev_block_header
    }

    pub fn chain_mmr(&self) -> &ChainMmr {
        &self.chain_mmr
    }

    pub fn into_parts(
        self,
    ) -> (
        Vec<ProvenBatch>,
        Vec<(AccountId, AccountUpdateWitness)>,
        BTreeMap<usize, Digest>,
        BTreeMap<Nullifier, SmtProof>,
        ChainMmr,
        BlockHeader,
    ) {
        (
            self.batches,
            self.updated_accounts,
            self.batch_created_notes_roots,
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

fn check_duplicate_input_notes(batches: &[ProvenBatch]) -> Result<(), ProposedBlockError> {
    let mut input_note_set = BTreeMap::new();

    for batch in batches {
        for input_note in batch.input_notes().iter() {
            if let Some(first_batch_id) = input_note_set.insert(input_note.nullifier(), batch.id())
            {
                return Err(ProposedBlockError::DuplicateInputNote {
                    note_nullifier: input_note.nullifier(),
                    first_batch_id,
                    second_batch_id: batch.id(),
                });
            }
        }
    }

    Ok(())
}

fn check_duplicate_output_notes(batches: &[ProvenBatch]) -> Result<(), ProposedBlockError> {
    let mut input_note_set = BTreeMap::new();

    for batch in batches {
        for output_note in batch.output_notes().iter() {
            if let Some(first_batch_id) = input_note_set.insert(output_note.id(), batch.id()) {
                return Err(ProposedBlockError::DuplicateOutputNote {
                    note_id: output_note.id(),
                    first_batch_id,
                    second_batch_id: batch.id(),
                });
            }
        }
    }

    Ok(())
}

/// Check that each nullifier in the block has a proof provided and that the nullifier is
/// unspent. The proofs are required to update the nullifier tree.
fn check_nullifiers(
    block_inputs: &BlockInputs,
    batches: &[ProvenBatch],
) -> Result<(), ProposedBlockError> {
    for nullifier in batches.iter().flat_map(ProvenBatch::produced_nullifiers) {
        match block_inputs.nullifiers().get(&nullifier) {
            Some(proof) => {
                let (_, nullifier_value) = proof
                    .leaf()
                    .entries()
                    .iter()
                    .find(|(key, _)| *key == nullifier.inner())
                    .ok_or(ProposedBlockError::NullifierProofMissing(nullifier))?;

                if *nullifier_value != PartialNullifierTree::UNSPENT_NULLIFIER_VALUE {
                    return Err(ProposedBlockError::NullifierSpent(nullifier));
                }
            },
            None => return Err(ProposedBlockError::NullifierProofMissing(nullifier)),
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
            return Err(ProposedBlockError::BatchRefernceBlockMissingFromChain {
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
) -> Result<(Vec<(AccountId, AccountUpdateWitness)>, Vec<BlockAccountUpdate>), ProposedBlockError> {
    // TODO: A HashMap would be much more efficient here as we don't need the order. We also
    // rebalance the tree when removing the updates which is also unnecessary.

    // Aggregate all updates for the same account and store each update indexed by its initial
    // state commitment so we can easily retrieve them later.
    // This let's us chronologically order the updates per account across batches.
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
            .ok_or(ProposedBlockError::MissingAccountInput(account_id))?;

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
                proof,
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
#[derive(Debug, PartialEq, Eq)]
pub struct AccountUpdateWitness {
    initial_state_commitment: Digest,
    final_state_commitment: Digest,
    proof: MerklePath,
    transactions: Vec<TransactionId>,
}
