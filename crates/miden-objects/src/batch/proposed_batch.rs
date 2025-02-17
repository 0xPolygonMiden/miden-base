use alloc::{
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
    sync::Arc,
    vec::Vec,
};

use crate::{
    account::AccountId,
    batch::{BatchAccountUpdate, BatchId, BatchNoteTree, InputOutputNoteTracker},
    block::{BlockHeader, BlockNumber},
    errors::ProposedBatchError,
    note::{NoteId, NoteInclusionProof},
    transaction::{ChainMmr, InputNoteCommitment, InputNotes, OutputNote, ProvenTransaction},
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    MAX_ACCOUNTS_PER_BATCH, MAX_INPUT_NOTES_PER_BATCH, MAX_OUTPUT_NOTES_PER_BATCH,
};

/// A proposed batch of transactions with all necessary data to validate it.
///
/// See [`ProposedBatch::new`] for what a proposed batch expects and guarantees.
///
/// This type is fairly large, so consider boxing it.
#[derive(Debug, Clone)]
pub struct ProposedBatch {
    /// The transactions of this batch.
    transactions: Vec<Arc<ProvenTransaction>>,
    /// The header is boxed as it has a large stack size.
    block_header: BlockHeader,
    /// The chain MMR used to authenticate:
    /// - all unauthenticated notes that can be authenticated,
    /// - all block hashes referenced by the transactions in the batch.
    chain_mmr: ChainMmr,
    /// The note inclusion proofs for unauthenticated notes that were consumed in the batch which
    /// can be authenticated.
    unauthenticated_note_proofs: BTreeMap<NoteId, NoteInclusionProof>,
    /// The ID of the batch, which is a cryptographic commitment to the transactions in the batch.
    id: BatchId,
    /// A map from account ID's updated in this batch to the aggregated update from all
    /// transaction's that touched the account.
    account_updates: BTreeMap<AccountId, BatchAccountUpdate>,
    /// The block number at which the batch will expire. This is the minimum of all transaction's
    /// expiration block number.
    batch_expiration_block_num: BlockNumber,
    /// The input note commitment of the transaction batch. This consists of all authenticated
    /// notes that transactions in the batch consume as well as unauthenticated notes whose
    /// authentication is delayed to the block kernel.
    input_notes: InputNotes<InputNoteCommitment>,
    /// The SMT over the output notes of this batch.
    output_notes_tree: BatchNoteTree,
    /// The output notes of this batch. This consists of all notes created by transactions in the
    /// batch that are not consumed within the same batch.
    output_notes: Vec<OutputNote>,
}

impl ProposedBatch {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`ProposedBatch`] from the provided parts.
    ///
    /// # Inputs
    ///
    /// - The given transactions must be correctly ordered. That is, if two transactions A and B
    ///   update the same account in this order, meaning A's initial account state commitment
    ///   matches the account state before any transactions are executed and B's initial account
    ///   state commitment matches the final account state commitment of A, then A must come before
    ///   B.
    /// - The chain MMR should contain all block headers
    ///   - that are referenced by note inclusion proofs in `unauthenticated_note_proofs`.
    ///   - that are referenced by a transaction in the batch.
    /// - The `unauthenticated_note_proofs` should contain [`NoteInclusionProof`]s for any
    ///   unauthenticated note consumed by the transaction's in the batch which can be
    ///   authenticated. This means it is not required that every unauthenticated note has an entry
    ///   in this map for two reasons.
    ///     - Unauthenticated note authentication can be delayed to the block kernel.
    ///     - Another transaction in the batch creates an output note matching an unauthenticated
    ///       input note, in which case inclusion in the chain does not need to be proven.
    /// - The block header's block number must be greater or equal to the highest block number
    ///   referenced by any transaction. This is not verified explicitly, but will implicitly cause
    ///   an error during validating that each reference block of a transaction is in the chain MMR.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// - The number of input notes exceeds [`MAX_INPUT_NOTES_PER_BATCH`].
    ///   - Note that unauthenticated notes that are created in the same batch do not count. Any
    ///     other input notes, unauthenticated or not, do count.
    /// - The number of output notes exceeds [`MAX_OUTPUT_NOTES_PER_BATCH`].
    ///   - Note that output notes that are consumed in the same batch as unauthenticated input
    ///     notes do not count.
    /// - Any note is consumed more than once.
    /// - Any note is created more than once.
    /// - The number of account updates exceeds [`MAX_ACCOUNTS_PER_BATCH`].
    ///   - Note that any number of transactions against the same account count as one update.
    /// - The chain MMRs chain length does not match the block header's block number. This means the
    ///   chain MMR should not contain the block header itself as it is added to the MMR in the
    ///   batch kernel.
    /// - The chain MMRs hashed peaks do not match the block header's chain root.
    /// - The reference block of any transaction is not in the chain MMR.
    /// - The note inclusion proof for an unauthenticated note fails to verify.
    /// - The block referenced by a note inclusion proof for an unauthenticated note is missing from
    ///   the chain MMR.
    /// - The transactions in the proposed batch which update the same account are not correctly
    ///   ordered.
    /// - The provided list of transactions is empty. An empty batch is pointless and would
    ///   potentially result in the same [`BatchId`] for two empty batches which would mean batch
    ///   IDs are no longer unique.
    /// - There are duplicate transactions.
    pub fn new(
        transactions: Vec<Arc<ProvenTransaction>>,
        block_header: BlockHeader,
        chain_mmr: ChainMmr,
        unauthenticated_note_proofs: BTreeMap<NoteId, NoteInclusionProof>,
    ) -> Result<Self, ProposedBatchError> {
        // Check for empty or duplicate transactions.
        // --------------------------------------------------------------------------------------------

        if transactions.is_empty() {
            return Err(ProposedBatchError::EmptyTransactionBatch);
        }

        let mut transaction_set = BTreeSet::new();
        for tx in transactions.iter() {
            if !transaction_set.insert(tx.id()) {
                return Err(ProposedBatchError::DuplicateTransaction { transaction_id: tx.id() });
            }
        }

        // Verify block header and chain MMR match.
        // --------------------------------------------------------------------------------------------

        if chain_mmr.chain_length() != block_header.block_num() {
            return Err(ProposedBatchError::InconsistentChainLength {
                expected: block_header.block_num(),
                actual: chain_mmr.chain_length(),
            });
        }

        let hashed_peaks = chain_mmr.peaks().hash_peaks();
        if hashed_peaks != block_header.chain_root() {
            return Err(ProposedBatchError::InconsistentChainRoot {
                expected: block_header.chain_root(),
                actual: hashed_peaks,
            });
        }

        // Verify all block references from the transactions are in the chain.
        // --------------------------------------------------------------------------------------------

        // Aggregate block references into a set since the chain MMR does not index by hash.
        let mut block_references =
            BTreeSet::from_iter(chain_mmr.block_headers().map(BlockHeader::hash));
        // Insert the block referenced by the batch to consider it authenticated. We can assume this
        // because the block kernel will verify the block hash as it is a public input to the batch
        // kernel.
        block_references.insert(block_header.hash());

        for tx in transactions.iter() {
            if !block_references.contains(&tx.block_ref()) {
                return Err(ProposedBatchError::MissingTransactionBlockReference {
                    block_reference: tx.block_ref(),
                    transaction_id: tx.id(),
                });
            }
        }

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
                    let batch_account_update = BatchAccountUpdate::from_transaction(tx);
                    vacant.insert(batch_account_update);
                },
                Entry::Occupied(occupied) => {
                    // This returns an error if the transactions are not correctly ordered, e.g. if
                    // B comes before A.
                    occupied.into_mut().merge_proven_tx(tx).map_err(|source| {
                        ProposedBatchError::AccountUpdateError {
                            account_id: tx.account_id(),
                            source,
                        }
                    })?;
                },
            };

            // The expiration block of the batch is the minimum of all transaction's expiration
            // block.
            batch_expiration_block_num = batch_expiration_block_num.min(tx.expiration_block_num());
        }

        if account_updates.len() > MAX_ACCOUNTS_PER_BATCH {
            return Err(ProposedBatchError::TooManyAccountUpdates(account_updates.len()));
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
                    return Err(ProposedBatchError::DuplicateInputNote {
                        note_nullifier: nullifier,
                        first_transaction_id,
                        second_transaction_id: tx.id(),
                    });
                }
            }
        }

        // Create input and output note set of the batch.
        // --------------------------------------------------------------------------------------------

        // Check for duplicate output notes and remove all output notes from the batch output note
        // set that are consumed by transactions.
        let (input_notes, output_notes) = InputOutputNoteTracker::from_transactions(
            transactions.iter().map(AsRef::as_ref),
            &unauthenticated_note_proofs,
            &chain_mmr,
            &block_header,
        )?;

        if input_notes.len() > MAX_INPUT_NOTES_PER_BATCH {
            return Err(ProposedBatchError::TooManyInputNotes(input_notes.len()));
        }
        // SAFETY: This is safe as we have checked for duplicates and the max number of input notes
        // in a batch.
        let input_notes = InputNotes::new_unchecked(input_notes);

        if output_notes.len() > MAX_OUTPUT_NOTES_PER_BATCH {
            return Err(ProposedBatchError::TooManyOutputNotes(output_notes.len()));
        }

        // Build the output notes SMT.
        // --------------------------------------------------------------------------------------------

        // SAFETY: We can `expect` here because:
        // - the input output note tracker already returns an error for duplicate output notes,
        // - we have checked that the number of output notes is <= 2^BATCH_NOTE_TREE_DEPTH.
        let output_notes_tree = BatchNoteTree::with_contiguous_leaves(
            output_notes.iter().map(|note| (note.id(), note.metadata())),
        )
        .expect("there should be no duplicate notes and there should be <= 2^BATCH_NOTE_TREE_DEPTH notes");

        // Compute batch ID.
        // --------------------------------------------------------------------------------------------

        let id = BatchId::from_transactions(transactions.iter().map(AsRef::as_ref));

        Ok(Self {
            id,
            transactions,
            block_header,
            chain_mmr,
            unauthenticated_note_proofs,
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

    /// Returns the [`InputNotes`] of this batch.
    pub fn input_notes(&self) -> &InputNotes<InputNoteCommitment> {
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
        BlockHeader,
        ChainMmr,
        BTreeMap<NoteId, NoteInclusionProof>,
        BatchId,
        BTreeMap<AccountId, BatchAccountUpdate>,
        InputNotes<InputNoteCommitment>,
        BatchNoteTree,
        Vec<OutputNote>,
        BlockNumber,
    ) {
        (
            self.transactions,
            self.block_header,
            self.chain_mmr,
            self.unauthenticated_note_proofs,
            self.id,
            self.account_updates,
            self.input_notes,
            self.output_notes_tree,
            self.output_notes,
            self.batch_expiration_block_num,
        )
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for ProposedBatch {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.transactions
            .iter()
            .map(|tx| tx.as_ref().clone())
            .collect::<Vec<ProvenTransaction>>()
            .write_into(target);

        self.block_header.write_into(target);
        self.chain_mmr.write_into(target);
        self.unauthenticated_note_proofs.write_into(target);
    }
}

impl Deserializable for ProposedBatch {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let transactions = Vec::<ProvenTransaction>::read_from(source)?
            .iter()
            .map(|tx| Arc::new(tx.clone()))
            .collect::<Vec<Arc<ProvenTransaction>>>();

        let block_header = BlockHeader::read_from(source)?;
        let chain_mmr = ChainMmr::read_from(source)?;
        let unauthenticated_note_proofs =
            BTreeMap::<NoteId, NoteInclusionProof>::read_from(source)?;

        ProposedBatch::new(transactions, block_header, chain_mmr, unauthenticated_note_proofs)
            .map_err(|source| {
                DeserializationError::UnknownError(format!(
                    "failed to create proposed batch: {source}"
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use miden_crypto::merkle::{Mmr, PartialMmr};
    use miden_verifier::ExecutionProof;
    use winter_air::proof::Proof;
    use winter_rand_utils::rand_array;

    use super::*;
    use crate::{
        account::{AccountIdVersion, AccountStorageMode, AccountType},
        transaction::ProvenTransactionBuilder,
        Digest, Word,
    };

    #[test]
    fn proposed_batch_serialization() {
        // create chain MMR with 3 blocks - i.e., 2 peaks
        let mut mmr = Mmr::default();
        for i in 0..3 {
            let block_header = BlockHeader::mock(i, None, None, &[], Digest::default());
            mmr.add(block_header.hash());
        }
        let partial_mmr: PartialMmr = mmr.peaks().into();
        let chain_mmr = ChainMmr::new(partial_mmr, Vec::new()).unwrap();

        let chain_root = chain_mmr.peaks().hash_peaks();
        let note_root: Word = rand_array();
        let kernel_root: Word = rand_array();
        let header =
            BlockHeader::mock(3, Some(chain_root), Some(note_root.into()), &[], kernel_root.into());

        let account_id = AccountId::dummy(
            [1; 15],
            AccountIdVersion::Version0,
            AccountType::FungibleFaucet,
            AccountStorageMode::Private,
        );
        let initial_account_hash =
            [2; 32].try_into().expect("failed to create initial account hash");
        let final_account_hash = [3; 32].try_into().expect("failed to create final account hash");
        let block_num = BlockNumber::from(1);
        let block_ref = header.hash();
        let expiration_block_num = BlockNumber::from(2);
        let proof = ExecutionProof::new(Proof::new_dummy(), Default::default());

        let tx = ProvenTransactionBuilder::new(
            account_id,
            initial_account_hash,
            final_account_hash,
            block_num,
            block_ref,
            expiration_block_num,
            proof,
        )
        .build()
        .expect("failed to build proven transaction");

        let batch =
            ProposedBatch::new(vec![Arc::new(tx)], header, chain_mmr, BTreeMap::new()).unwrap();

        let encoded_batch = batch.to_bytes();

        let batch2 = ProposedBatch::read_from_bytes(&encoded_batch).unwrap();

        assert_eq!(batch.transactions(), batch2.transactions());
        assert_eq!(batch.block_header, batch2.block_header);
        assert_eq!(batch.chain_mmr, batch2.chain_mmr);
        assert_eq!(batch.unauthenticated_note_proofs, batch2.unauthenticated_note_proofs);
        assert_eq!(batch.id, batch2.id);
        assert_eq!(batch.account_updates, batch2.account_updates);
        assert_eq!(batch.batch_expiration_block_num, batch2.batch_expiration_block_num);
        assert_eq!(batch.input_notes, batch2.input_notes);
        assert_eq!(batch.output_notes, batch2.output_notes);
        assert_eq!(batch.output_notes_tree, batch2.output_notes_tree);
    }
}
