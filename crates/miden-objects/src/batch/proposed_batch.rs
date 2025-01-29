use alloc::{
    boxed::Box,
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
    sync::Arc,
    vec::Vec,
};

use crate::{
    account::AccountId,
    batch::{BatchAccountUpdate, BatchId, BatchNoteTree},
    block::{BlockHeader, BlockNumber},
    errors::BatchError,
    note::{NoteHeader, NoteId, NoteInclusionProof},
    transaction::{ChainMmr, InputNoteCommitment, OutputNote, ProvenTransaction, TransactionId},
};

/// A proposed batch of transactions with all necessary data to validate it.
#[derive(Debug, Clone)]
pub struct ProposedBatch {
    transactions: Vec<Arc<ProvenTransaction>>,
    /// The header is boxed as it has a large stack size.
    block_header: Box<BlockHeader>,
    /// The chain MMR used to authenticate:
    /// - all unauthenticated notes that can be authenticated,
    /// - all block hashes referenced by the transactions in the batch.
    block_chain: ChainMmr,
    /// The note inclusion proofs for unauthenticated notes that were consumed in the batch which
    /// can be authenticated.
    authenticatable_unauthenticated_notes: BTreeMap<NoteId, NoteInclusionProof>,
    id: BatchId,
    account_updates: BTreeMap<AccountId, BatchAccountUpdate>,
    batch_expiration_block_num: BlockNumber,
    input_notes: Vec<InputNoteCommitment>,
    output_notes_tree: BatchNoteTree,
    output_notes: Vec<OutputNote>,
}

impl ProposedBatch {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`ProposedBatch`] from the provided parts.
    ///
    /// # Inputs
    ///
    /// - The chain MMR should contain all block headers
    ///   - that are referenced by note inclusion proofs in `authenticatable_unauthenticated_notes`.
    ///   - that are referenced by a transaction in the batch.
    /// - The `authenticatable_unauthenticated_notes` should contain [`NoteInclusionProof`]s for any
    ///   unauthenticated note consumed by the transaction's in the batch which can be
    ///   authenticated. This means it is not required that every unauthenticated note has an entry
    ///   in this map for two reasons.
    ///     - Unauthenticated note authentication can be delayed to the block kernel.
    ///     - Another transaction in the batch produces the unauthenticated input note, in which
    ///       case inclusion in the chain must not be proven.
    /// - The block header's block number must be greater or equal to the highest block number
    ///   referenced by any transaction. This is not verified explicitly, but will implicitly cause
    ///   an error during validating that each reference block of a transaction is in the chain MMR.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// - The chain MMRs chain length does not match the block header's block number. This means the
    ///   chain MMR should not contain the block header itself as it is added to the MMR in the
    ///   batch kernel.
    /// - The chain MMRs hashed peaks do not match the block header's chain root.
    /// - The reference block of any transaction is not in the chain MMR.
    /// - The note inclusion proof for an unauthenticated note fails to verify.
    /// - The block referenced by a note inclusion proof for an unauthenticated note is missing from
    ///   the chain MMR.
    /// - The transactions in the proposed batch which update the same account are not correctly
    ///   ordered. That is, if two transactions A and B update the same account in this order,
    ///   meaning B's initial account state commitment matches the final account state commitment of
    ///   A, then A must come before B in the [`ProposedBatch`].
    /// - Any note is consumed twice.
    /// - Any note is created more than once.
    pub fn new(
        transactions: Vec<Arc<ProvenTransaction>>,
        block_header: BlockHeader,
        chain_mmr: ChainMmr,
        authenticatable_unauthenticated_notes: BTreeMap<NoteId, NoteInclusionProof>,
    ) -> Result<Self, BatchError> {
        // TODO: Check max num tranactions in batch.

        // Verify block header and chain MMR match.
        // --------------------------------------------------------------------------------------------

        if chain_mmr.chain_length() != block_header.block_num() {
            return Err(BatchError::InconsistentChainLength {
                expected: block_header.block_num(),
                actual: chain_mmr.chain_length(),
            });
        }

        let hashed_peaks = chain_mmr.peaks().hash_peaks();
        if hashed_peaks != block_header.chain_root() {
            return Err(BatchError::InconsistentChainRoot {
                expected: block_header.chain_root(),
                actual: hashed_peaks,
            });
        }

        // Verify all block references from the transactions are in the chain.
        // --------------------------------------------------------------------------------------------

        // Aggregate block references into a set since the chain MMR does not index by hash.
        let mut block_references =
            BTreeSet::from_iter(chain_mmr.block_headers_iter().map(BlockHeader::hash));
        // Insert the block referenced by the batch to consider it authenticated. We can assume this
        // because the block kernel will verify the block hash as it is a public input to the batch
        // kernel.
        block_references.insert(block_header.hash());

        for tx in transactions.iter() {
            if !block_references.contains(&tx.block_ref()) {
                return Err(BatchError::MissingTransactionBlockReference {
                    block_reference: tx.block_ref(),
                    transaction_id: tx.id(),
                });
            }
        }

        // Compute batch ID.
        // --------------------------------------------------------------------------------------------

        let id = BatchId::compute(transactions.iter().map(|tx| tx.id()));

        // Aggregate individual tx-level account updates into a batch-level account update - one per
        // account.
        // --------------------------------------------------------------------------------------------

        // Populate batch output notes and updated accounts.
        let mut account_updates = BTreeMap::<AccountId, BatchAccountUpdate>::new();
        let mut batch_expiration_block_num = BlockNumber::from(u32::MAX);
        for tx in transactions.iter() {
            // Merge account updates so that state transitions A->B->C become A->C.
            match account_updates.entry(tx.account_id()) {
                Entry::Vacant(vacant) => {
                    let batch_account_update = BatchAccountUpdate::new(
                        tx.account_id(),
                        tx.account_update().init_state_hash(),
                        tx.account_update().final_state_hash(),
                        vec![tx.id()],
                        tx.account_update().details().clone(),
                    );
                    vacant.insert(batch_account_update);
                },
                Entry::Occupied(occupied) => {
                    // This returns an error if the transactions are not correctly ordered, e.g. if
                    // B comes before A.
                    occupied.into_mut().merge_proven_tx(tx).map_err(|source| {
                        BatchError::AccountUpdateError { account_id: tx.account_id(), source }
                    })?;
                },
            };

            // The expiration block of the batch is the minimum of all transaction's expiration
            // block.
            batch_expiration_block_num = batch_expiration_block_num.min(tx.expiration_block_num());
        }

        // Check for duplicates in input notes.
        // --------------------------------------------------------------------------------------------

        // Check for duplicate input notes both within a transaction and across transactions.
        // This also includes authenticated notes, as the transaction kernel doesn't check for
        // duplicates.
        let mut input_note_map = BTreeMap::new();

        for tx in transactions.iter() {
            for note in tx.input_notes() {
                let nullifier = note.nullifier();
                if let Some(first_transaction_id) = input_note_map.insert(nullifier, tx.id()) {
                    return Err(BatchError::DuplicateInputNote {
                        note_nullifier: nullifier,
                        first_transaction_id,
                        second_transaction_id: tx.id(),
                    });
                }
            }
        }

        // Create input and output note set of the batch.
        // --------------------------------------------------------------------------------------------

        // Remove all output notes from the batch output note set that are consumed by transactions.
        //
        // This still allows transaction `A` to consume an unauthenticated note `x` and output note
        // `y` and for transaction `B` to consume an unauthenticated note `y` and output
        // note `x` (i.e., have a circular dependency between transactions).
        let mut output_notes = BatchOutputNoteTracker::new(transactions.iter().map(AsRef::as_ref))?;
        let mut input_notes = vec![];

        for tx in transactions.iter() {
            for input_note in tx.input_notes().iter() {
                // Header is present only for unauthenticated input notes.
                let input_note = match input_note.header() {
                    Some(input_note_header) => {
                        if output_notes.remove_note(input_note_header)? {
                            // If a transaction consumes a note that is also created in this batch,
                            // it is removed from the set of output notes.
                            // We `continue` so that the input note is not added to the set of input
                            // notes of the batch.
                            // That way the note appears in neither input nor output set.
                            continue;
                        }

                        // If an inclusion proof for an unauthenticated note is provided and the
                        // proof is valid, it means the note is part of the chain and we can mark it
                        // as authenticated by erasing the note header.
                        if let Some(proof) =
                            authenticatable_unauthenticated_notes.get(&input_note_header.id())
                        {
                            let note_block_header =
                                chain_mmr.get_block(proof.location().block_num()).ok_or_else(
                                    || BatchError::UnauthenticatedInputNoteBlockNotInChainMmr {
                                        block_number: proof.location().block_num(),
                                        note_id: input_note_header.id(),
                                    },
                                )?;

                            authenticate_unauthenticated_note(
                                input_note_header,
                                proof,
                                note_block_header,
                            )?;

                            // Erase the note header from the input note.
                            InputNoteCommitment::from(input_note.nullifier())
                        } else {
                            input_note.clone()
                        }
                    },
                    None => input_note.clone(),
                };
                input_notes.push(input_note);
            }
        }

        let output_notes = output_notes.into_notes();

        // Build the output notes SMT.
        // --------------------------------------------------------------------------------------------

        let output_notes_tree = BatchNoteTree::with_contiguous_leaves(
            output_notes.iter().map(|note| (note.id(), note.metadata())),
        )
        .expect("output note tracker should return an error for duplicate notes");

        Ok(Self {
            id,
            transactions,
            block_header: Box::new(block_header),
            block_chain: chain_mmr,
            authenticatable_unauthenticated_notes,
            account_updates,
            batch_expiration_block_num,
            input_notes,
            output_notes,
            output_notes_tree,
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a slice of the [`ProvenTransaction`]s in the batch.
    pub fn transactions(&self) -> &[Arc<ProvenTransaction>] {
        &self.transactions
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

    /// The ID of this batch. See [`BatchId`] for details on how it is computed.
    pub fn id(&self) -> BatchId {
        self.id
    }

    /// Returns the block number at which the batch will expire.
    pub fn batch_expiration_block_num(&self) -> BlockNumber {
        self.batch_expiration_block_num
    }

    /// Returns the slice of [`InputNoteCommitment`]s of this batch.
    pub fn input_notes(&self) -> &[InputNoteCommitment] {
        &self.input_notes
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
        &self.output_notes_tree
    }

    /// Consumes the proposed batch and returns its underlying parts.
    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        Vec<Arc<ProvenTransaction>>,
        Box<BlockHeader>,
        ChainMmr,
        BTreeMap<NoteId, NoteInclusionProof>,
        BatchId,
        BTreeMap<AccountId, BatchAccountUpdate>,
        Vec<InputNoteCommitment>,
        BatchNoteTree,
        Vec<OutputNote>,
        BlockNumber,
    ) {
        (
            self.transactions,
            self.block_header,
            self.block_chain,
            self.authenticatable_unauthenticated_notes,
            self.id,
            self.account_updates,
            self.input_notes,
            self.output_notes_tree,
            self.output_notes,
            self.batch_expiration_block_num,
        )
    }
}

// BATCH OUTPUT NOTE TRACKER
// ================================================================================================

/// A helper struct to track output notes.
/// Its main purpose is to check for duplicates and allow for removal of output notes that are
/// consumed in the same batch, so are not output notes of the batch.
///
/// The approach for this is that the output note set is initialized to the union of all output
/// notes of the transactions in the batch.
/// Then (outside of this struct) all input notes of transactions in the batch which are also output
/// notes can be removed, as they are considered consumed within the batch and will not be visible
/// as created or consumed notes for the batch.
#[derive(Debug)]
struct BatchOutputNoteTracker {
    /// An index from [`NoteId`]s to the transaction that creates the note and the note itself.
    /// The transaction ID is tracked to produce better errors when a duplicate note is
    /// encountered.
    output_notes: BTreeMap<NoteId, (TransactionId, OutputNote)>,
}

impl BatchOutputNoteTracker {
    /// Constructs a new output note tracker from the given transactions.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - any output note is created more than once (by the same or different transactions).
    fn new<'a>(txs: impl Iterator<Item = &'a ProvenTransaction>) -> Result<Self, BatchError> {
        let mut output_notes = BTreeMap::new();
        for tx in txs {
            for note in tx.output_notes().iter() {
                if let Some((first_transaction_id, _)) =
                    output_notes.insert(note.id(), (tx.id(), note.clone()))
                {
                    return Err(BatchError::DuplicateOutputNote {
                        note_id: note.id(),
                        first_transaction_id,
                        second_transaction_id: tx.id(),
                    });
                }
            }
        }

        Ok(Self { output_notes })
    }

    /// Attempts to remove the given input note from the output note set.
    ///
    /// Returns `true` if the given note existed in the output note set and was removed from it,
    /// `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the given note has a corresponding note in the output note set with the same [`NoteId`]
    ///   but their hashes differ (i.e. their metadata is different).
    pub fn remove_note(&mut self, input_note_header: &NoteHeader) -> Result<bool, BatchError> {
        let id = input_note_header.id();
        if let Some((_, output_note)) = self.output_notes.remove(&id) {
            // Check if the notes with the same ID have differing hashes.
            // This could happen if the metadata of the notes is different, which we consider an
            // error.
            let input_hash = input_note_header.hash();
            let output_hash = output_note.hash();
            if output_hash != input_hash {
                return Err(BatchError::NoteHashesMismatch { id, input_hash, output_hash });
            }

            return Ok(true);
        }

        Ok(false)
    }

    /// Consumes the tracker and returns a [`Vec`] of output notes sorted by [`NoteId`].
    pub fn into_notes(self) -> Vec<OutputNote> {
        self.output_notes.into_iter().map(|(_, (_, output_note))| output_note).collect()
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Validates whether the provided unauthenticated note belongs to the note tree of the specified
/// block header.
fn authenticate_unauthenticated_note(
    note_header: &NoteHeader,
    proof: &NoteInclusionProof,
    block_header: &BlockHeader,
) -> Result<(), BatchError> {
    let note_index = proof.location().node_index_in_block().into();
    let note_hash = note_header.hash();
    proof
        .note_path()
        .verify(note_index, note_hash, &block_header.note_root())
        .map_err(|source| BatchError::UnauthenticatedNoteAuthenticationFailed {
            note_id: note_header.id(),
            block_num: proof.location().block_num(),
            source,
        })
}
