use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};

use crate::{
    account::{delta::AccountUpdateDetails, AccountId},
    batch::{BatchAccountUpdate, BatchId, InputOutputNoteTracker, ProvenBatch},
    block::{
        block_inputs::BlockInputs, AccountUpdateWitness, AccountWitness, BlockHeader, BlockNumber,
        NullifierWitness, OutputNoteBatch,
    },
    errors::ProposedBlockError,
    note::{NoteId, Nullifier},
    transaction::{ChainMmr, InputNoteCommitment, OutputNote, TransactionId},
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest, EMPTY_WORD, MAX_BATCHES_PER_BLOCK,
};

// PROPOSED BLOCK
// =================================================================================================

/// A proposed block with many, but not all constraints of a
/// [`ProvenBlock`](crate::block::ProvenBlock) enforced.
///
/// See [`ProposedBlock::new_at`] for details on the checks.
#[derive(Debug, Clone)]
pub struct ProposedBlock {
    /// The transaction batches in this block.
    batches: Vec<ProvenBatch>,
    /// The unix timestamp of the block in seconds.
    timestamp: u32,
    /// All account's [`AccountUpdateWitness`] that were updated in this block. See its docs for
    /// details.
    account_updated_witnesses: Vec<(AccountId, AccountUpdateWitness)>,
    /// Note batches created by the transactions in this block.
    ///
    /// These are the output notes after note erasure has been done, so they represent the actual
    /// output notes of the block.
    ///
    /// The length of this vector is guaranteed to be equal to the length of `batches` and the
    /// inner batch of output notes may be empty if a batch did not create any notes.
    output_note_batches: Vec<OutputNoteBatch>,
    /// The nullifiers created by this block.
    ///
    /// These are the nullifiers of all input notes after note erasure has been done, so these are
    /// the nullifiers of all _authenticated_ notes consumed in the block.
    created_nullifiers: BTreeMap<Nullifier, NullifierWitness>,
    /// The [`ChainMmr`] at the state of the previous block header. It is used to:
    /// - authenticate unauthenticated notes whose note inclusion proof references a block.
    /// - authenticate all reference blocks of the batches in this block.
    chain_mmr: ChainMmr,
    /// The previous block's header which this block builds on top of.
    ///
    /// As part of proving the block, this header will be added to the next chain MMR.
    prev_block_header: BlockHeader,
}

impl ProposedBlock {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new proposed block from the provided [`BlockInputs`], transaction batches and
    /// timestamp.
    ///
    /// This checks most of the constraints of a block and computes most of the data structure
    /// updates except for the more expensive tree updates (nullifier, account and chain root).
    ///
    /// # Errors
    ///
    /// Returns an error if any of the following conditions are met.
    ///
    /// ## Batches
    ///
    /// - The number of batches exceeds [`MAX_BATCHES_PER_BLOCK`].
    /// - There are duplicate batches, i.e. they have the same [`BatchId`].
    /// - The expiration block number of any batch is less than the block number of the currently
    ///   proposed block.
    ///
    /// ## Chain
    ///
    /// - The length of the [`ChainMmr`] in the block inputs is not equal to the previous block
    ///   header in the block inputs.
    /// - The [`ChainMmr`]'s chain root is not equal to the [`BlockHeader::chain_root`] of the
    ///   previous block header.
    ///
    /// ## Notes
    ///
    /// Note that, in the following, the set of authenticated notes includes unauthenticated notes
    /// that have been authenticated.
    ///
    /// - The union of all input notes across all batches contain duplicates.
    /// - The union of all output notes across all batches contain duplicates.
    /// - There is an unauthenticated input note and an output note with the same note ID but their
    ///   note hashes are different (i.e. their metadata is different).
    /// - There is a note inclusion proof for an unauthenticated note whose referenced block is not
    ///   in the [`ChainMmr`].
    /// - The note inclusion proof for an unauthenticated is invalid.
    /// - There are any unauthenticated notes for which no note inclusion proof is provided.
    /// - A [`NullifierWitness`] is missing for an authenticated note.
    /// - If the [`NullifierWitness`] for an authenticated note proves that the note was already
    ///   consumed.
    ///
    /// ## Accounts
    ///
    /// - An [`AccountWitness`] is missing for an account updated by a batch.
    /// - Any two batches update the same account from the same state. For example, if batch 1
    ///   updates some account from state A to B and batch 2 updates it from A to F, then those
    ///   batches conflict as they both start from the same initial state but produce a fork in the
    ///   account's state.
    /// - Account updates from different batches cannot be brought in a contiguous order. For
    ///   example, if a batch 1 updates an account from state A to C, and a batch 2 updates it from
    ///   D to F, then the state transition from C to D is missing. Note that this does not mean,
    ///   that batches must be provided in an order where account updates chain together in the
    ///   order of the batches, which would generally be an impossible requirement to fulfill.
    /// - Account updates cannot be merged, i.e. if [`AccountUpdateDetails::merge`] fails on the
    ///   updates from two batches.
    ///
    /// ## Time
    ///
    /// - The given `timestamp` does not increase monotonically compared to the previous block
    ///   header' timestamp.
    pub fn new_at(
        block_inputs: BlockInputs,
        batches: Vec<ProvenBatch>,
        timestamp: u32,
    ) -> Result<Self, ProposedBlockError> {
        // Check for duplicate and max number of batches.
        // --------------------------------------------------------------------------------------------

        if batches.len() > MAX_BATCHES_PER_BLOCK {
            return Err(ProposedBlockError::TooManyBatches);
        }

        check_duplicate_batches(&batches)?;

        // Check timestamp increases monotonically.
        // --------------------------------------------------------------------------------------------

        check_timestamp_increases_monotonically(timestamp, block_inputs.prev_block_header())?;

        // Check for batch expiration.
        // --------------------------------------------------------------------------------------------

        check_batch_expiration(&batches, block_inputs.prev_block_header())?;

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

        let (block_input_notes, block_erased_notes, block_output_notes) =
            InputOutputNoteTracker::from_batches(
                batches.iter(),
                block_inputs.unauthenticated_note_proofs(),
                block_inputs.chain_mmr(),
                block_inputs.prev_block_header(),
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

        let (prev_block_header, chain_mmr, account_witnesses, mut nullifier_witnesses, _) =
            block_inputs.into_parts();

        // Remove nullifiers of erased notes, so we only add the nullifiers of actual input notes to
        // the proposed block.
        remove_erased_nullifiers(&mut nullifier_witnesses, block_erased_notes.into_iter());

        // Check against computed block_input_notes which also contain unauthenticated notes that
        // have been authenticated.
        check_nullifiers(
            &nullifier_witnesses,
            block_input_notes.iter().map(InputNoteCommitment::nullifier),
        )?;

        // Aggregate account updates across batches.
        // --------------------------------------------------------------------------------------------

        let aggregator = AccountUpdateAggregator::from_batches(&batches)?;
        let account_updated_witnesses = aggregator.into_update_witnesses(account_witnesses)?;

        // Compute the block's output note batches from the individual batch output notes.
        // --------------------------------------------------------------------------------------------

        let output_note_batches = compute_block_output_notes(&batches, block_output_notes);

        // Build proposed blocks from parts.
        // --------------------------------------------------------------------------------------------

        Ok(Self {
            batches,
            timestamp,
            account_updated_witnesses,
            output_note_batches,
            created_nullifiers: nullifier_witnesses,
            chain_mmr,
            prev_block_header,
        })
    }

    /// Creates a new proposed block from the provided [`BlockInputs`] and transaction batches.
    ///
    /// Equivalent to [`ProposedBlock::new_at`] except that the timestamp of the proposed block is
    /// set to the current system time or the previous block header's timestamp + 1, whichever
    /// is greater. This guarantees that the timestamp increases monotonically.
    ///
    /// See the [`ProposedBlock::new_at`] for details on errors and other constraints.
    #[cfg(feature = "std")]
    pub fn new(
        block_inputs: BlockInputs,
        batches: Vec<ProvenBatch>,
    ) -> Result<Self, ProposedBlockError> {
        let timestamp_now: u32 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("now should be after 1970")
            .as_secs()
            .try_into()
            .expect("timestamp should fit in a u32 before the year 2106");

        let timestamp = timestamp_now.max(block_inputs.prev_block_header().timestamp() + 1);

        Self::new_at(block_inputs, batches, timestamp)
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns an iterator over all transactions which affected accounts in the block with
    /// corresponding account IDs.
    pub fn affected_accounts(&self) -> impl Iterator<Item = (TransactionId, AccountId)> + '_ {
        self.account_updated_witnesses.iter().flat_map(|(account_id, update)| {
            update.transactions().iter().map(move |tx_id| (*tx_id, *account_id))
        })
    }

    /// Returns the block number of this proposed block.
    pub fn block_num(&self) -> BlockNumber {
        // The chain length is the length at the state of the previous block header, so we have to
        // add one.
        self.chain_mmr().chain_length() + 1
    }

    /// Returns a reference to the slice of batches in this block.
    pub fn batches(&self) -> &[ProvenBatch] {
        &self.batches
    }

    /// Returns the map of nullifiers to their proofs from the proposed block.
    pub fn created_nullifiers(&self) -> &BTreeMap<Nullifier, NullifierWitness> {
        &self.created_nullifiers
    }

    /// Returns a reference to the previous block header that this block builds on top of.
    pub fn prev_block_header(&self) -> &BlockHeader {
        &self.prev_block_header
    }

    /// Returns the [`ChainMmr`] that this block contains.
    pub fn chain_mmr(&self) -> &ChainMmr {
        &self.chain_mmr
    }

    /// Returns a reference to the slice of accounts updated in this block.
    pub fn updated_accounts(&self) -> &[(AccountId, AccountUpdateWitness)] {
        &self.account_updated_witnesses
    }

    /// Returns the timestamp of this block.
    pub fn timestamp(&self) -> u32 {
        self.timestamp
    }

    /// Returns a slice of the [`OutputNoteBatch`] of each batch in this block.
    pub fn output_note_batches(&self) -> &[OutputNoteBatch] {
        &self.output_note_batches
    }

    // STATE MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Consumes self and returns the non-[`Copy`] parts of the block.
    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        Vec<ProvenBatch>,
        Vec<(AccountId, AccountUpdateWitness)>,
        Vec<OutputNoteBatch>,
        BTreeMap<Nullifier, NullifierWitness>,
        ChainMmr,
        BlockHeader,
    ) {
        (
            self.batches,
            self.account_updated_witnesses,
            self.output_note_batches,
            self.created_nullifiers,
            self.chain_mmr,
            self.prev_block_header,
        )
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for ProposedBlock {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.batches.write_into(target);
        self.timestamp.write_into(target);
        self.account_updated_witnesses.write_into(target);
        self.output_note_batches.write_into(target);
        self.created_nullifiers.write_into(target);
        self.chain_mmr.write_into(target);
        self.prev_block_header.write_into(target);
    }
}

impl Deserializable for ProposedBlock {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let block = Self {
            batches: <Vec<ProvenBatch>>::read_from(source)?,
            timestamp: u32::read_from(source)?,
            account_updated_witnesses: <Vec<(AccountId, AccountUpdateWitness)>>::read_from(source)?,
            output_note_batches: <Vec<OutputNoteBatch>>::read_from(source)?,
            created_nullifiers: <BTreeMap<Nullifier, NullifierWitness>>::read_from(source)?,
            chain_mmr: ChainMmr::read_from(source)?,
            prev_block_header: BlockHeader::read_from(source)?,
        };

        Ok(block)
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

fn check_timestamp_increases_monotonically(
    provided_timestamp: u32,
    prev_block_header: &BlockHeader,
) -> Result<(), ProposedBlockError> {
    if provided_timestamp <= prev_block_header.timestamp() {
        Err(ProposedBlockError::TimestampDoesNotIncreaseMonotonically {
            provided_timestamp,
            previous_timestamp: prev_block_header.timestamp(),
        })
    } else {
        Ok(())
    }
}

/// Checks whether any of the batches is expired and can no longer be included in this block.
///
/// To illustrate, a batch which expired at block 4 cannot be included in block 5, but if it
/// expires at block 5 then it can still be included in block 5.
fn check_batch_expiration(
    batches: &[ProvenBatch],
    prev_block_header: &BlockHeader,
) -> Result<(), ProposedBlockError> {
    let current_block_num = prev_block_header.block_num() + 1;

    for batch in batches {
        if batch.batch_expiration_block_num() < current_block_num {
            return Err(ProposedBlockError::ExpiredBatch {
                batch_id: batch.id(),
                batch_expiration_block_num: batch.batch_expiration_block_num(),
                current_block_num,
            });
        }
    }

    Ok(())
}

/// Check that each nullifier in the block has a proof provided and that the nullifier is
/// unspent. The proofs are required to update the nullifier tree.
fn check_nullifiers(
    nullifier_witnesses: &BTreeMap<Nullifier, NullifierWitness>,
    block_input_notes: impl Iterator<Item = Nullifier>,
) -> Result<(), ProposedBlockError> {
    for block_input_note in block_input_notes {
        match nullifier_witnesses
            .get(&block_input_note)
            .and_then(|x| x.proof().get(&block_input_note.inner()))
        {
            Some(nullifier_value) => {
                if nullifier_value != EMPTY_WORD {
                    return Err(ProposedBlockError::NullifierSpent(block_input_note));
                }
            },
            // If the nullifier witnesses did not contain a proof for this nullifier or the provided
            // proof was not for this nullifier, then it's an error.
            None => return Err(ProposedBlockError::NullifierProofMissing(block_input_note)),
        }
    }

    Ok(())
}

/// Removes the nullifiers from the nullifier witnesses that were erased (i.e. created and consumed
/// within the block).
fn remove_erased_nullifiers(
    nullifier_witnesses: &mut BTreeMap<Nullifier, NullifierWitness>,
    block_erased_notes: impl Iterator<Item = Nullifier>,
) {
    for erased_note in block_erased_notes {
        // We do not check that the nullifier was actually present to allow the block inputs to
        // not include a nullifier that is known to belong to an erased note.
        let _ = nullifier_witnesses.remove(&erased_note);
    }
}

/// Checks consistency between the previous block header and the provided chain MMR.
///
/// This checks that:
/// - the chain length of the chain MMR is equal to the block number of the previous block header,
///   i.e. the chain MMR's latest block is the previous' blocks reference block. The previous block
///   header will be added to the chain MMR as part of constructing the current block.
/// - the root of the chain MMR is equivalent to the chain root of the previous block header.
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

/// Computes the block's output notes from the batches of notes of each batch in the block.
///
/// We pass in `block_output_notes` which is the full set of output notes of the block, with output
/// notes erased that are consumed by some batch in the block.
///
/// The batch output notes of each proven batch however contain all the notes that it creates,
/// including ones that were potentially erased in `block_output_notes`. This means we have to
/// make the batch output notes consistent with `block_output_notes` by removing the erased notes.
/// Then it accurately represents what output notes the batch actually creates as part of the block.
///
/// Returns the set of [`OutputNoteBatch`]es that each batch creates.
fn compute_block_output_notes(
    batches: &[ProvenBatch],
    mut block_output_notes: BTreeMap<NoteId, (BatchId, OutputNote)>,
) -> Vec<OutputNoteBatch> {
    let mut block_output_note_batches = Vec::with_capacity(batches.len());

    for batch in batches.iter() {
        let batch_output_notes = compute_batch_output_notes(batch, &mut block_output_notes);
        block_output_note_batches.push(batch_output_notes);
    }

    block_output_note_batches
}

/// Computes the output note of the given batch. This is essentially the batch's output notes minus
/// all erased notes.
///
/// If a note in the batch's output notes is not present in the block output notes map it means it
/// was erased and should therefore not be added to the batch's output notes. If it is present, it
/// is added to the set of output notes of this batch.
///
/// The output note set is returned.
fn compute_batch_output_notes(
    batch: &ProvenBatch,
    block_output_notes: &mut BTreeMap<NoteId, (BatchId, OutputNote)>,
) -> OutputNoteBatch {
    // The len of the batch output notes is an upper bound of how many notes the batch could've
    // produced so we reserve that much space to avoid reallocation.
    let mut batch_output_notes = Vec::with_capacity(batch.output_notes().len());

    for (note_idx, original_output_note) in batch.output_notes().iter().enumerate() {
        // If block_output_notes no longer contains a note it means it was erased and we do not
        // include it in the output notes of the current batch. We include the original index of the
        // note in the batch so we can later correctly construct the block note tree. This index is
        // needed because we want to be able to construct the block note tree in two ways: 1) By
        // inserting the individual batch note trees (with erased notes removed) as subtrees into an
        // empty block note tree or 2) by iterating the set `OutputNoteBatch`es. If we did not store
        // the index, then the second method would assume a contiguous layout of output notes and
        // result in a different tree than the first method.
        //
        // Note that because we disallow duplicate output notes, if this map contains the
        // original note id, then we can be certain it was created by this batch and should stay
        // in the tree. In other words, there is no ambiguity where a note originated from.
        if let Some((_batch_id, output_note)) =
            block_output_notes.remove(&original_output_note.id())
        {
            debug_assert_eq!(_batch_id, batch.id(), "batch that contained the note originally is no longer the batch that contains it according to the provided map");
            batch_output_notes.push((note_idx, output_note));
        }
    }

    batch_output_notes
}

// ACCOUNT UPDATE AGGREGATOR
// ================================================================================================

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

    /// Aggregates all updates for the same account and stores each update indexed by its initial
    /// state commitment so we can easily retrieve them in the next step. This lets us
    /// chronologically order the updates per account across batches.
    fn from_batches(batches: &[ProvenBatch]) -> Result<Self, ProposedBlockError> {
        let mut update_aggregator = AccountUpdateAggregator::new();

        for batch in batches {
            for (account_id, update) in batch.account_updates() {
                update_aggregator.insert_update(*account_id, batch.id(), update.clone())?;
            }
        }

        Ok(update_aggregator)
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
    /// For each updated account an entry in `account_witnesses` must be present.
    fn into_update_witnesses(
        self,
        mut account_witnesses: BTreeMap<AccountId, AccountWitness>,
    ) -> Result<Vec<(AccountId, AccountUpdateWitness)>, ProposedBlockError> {
        let mut account_update_witnesses = Vec::with_capacity(self.updates.len());

        for (account_id, updates_map) in self.updates {
            let witness = account_witnesses
                .remove(&account_id)
                .ok_or(ProposedBlockError::MissingAccountWitness(account_id))?;

            let account_update_witness = Self::aggregate_account(account_id, witness, updates_map)?;

            account_update_witnesses.push((account_id, account_update_witness));
        }

        Ok(account_update_witnesses)
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
                ProposedBlockError::InconsistentAccountStateTransition {
                    account_id,
                    state_commitment: current_commitment,
                    remaining_state_commitments: updates.keys().copied().collect(),
                }
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
