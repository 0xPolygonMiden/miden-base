use alloc::{
    collections::{btree_map::Entry, BTreeMap},
    vec::Vec,
};

use miden_objects::{
    account::AccountId,
    batch::{BatchAccountUpdate, BatchId, BatchNoteTree},
    block::{BlockHeader, BlockNumber},
    note::{NoteHeader, NoteId, NoteInclusionProof},
    transaction::{InputNoteCommitment, OutputNote, ProvenTransaction, TransactionId},
};

use crate::{BatchError, ProposedBatch, ProvenBatch};

// LOCAL BATCH PROVER
// ================================================================================================

/// A local prover for transaction batches, turning a [`ProposedBatch`] into a [`ProvenBatch`].
pub struct LocalBatchProver {}

impl LocalBatchProver {
    /// Attempts to prove the [`ProposedBatch`] into a [`ProvenBatch`].
    ///
    /// Returns an error if:
    ///
    /// - The transactions in the proposed batch which update the same account are not correctly
    ///   ordered. That is, if two transactions A and B update the same account in this order,
    ///   meaning B's initial account state commitment matches the final account state commitment of
    ///   A, then A must come before B in the [`ProposedBatch`].
    /// - If any note is consumed twice.
    /// - If any note is created more than once.
    pub fn prove(proposed_batch: ProposedBatch) -> Result<ProvenBatch, BatchError> {
        let transactions = proposed_batch.transactions();
        let id = BatchId::compute(transactions.iter().map(|tx| tx.id()));

        // Aggregate individual tx-level account updates into a batch-level account update - one per
        // account.
        // --------------------------------------------------------------------------------------------

        // Populate batch output notes and updated accounts.
        let mut updated_accounts = BTreeMap::<AccountId, BatchAccountUpdate>::new();
        let mut batch_expiration_block_num = BlockNumber::from(u32::MAX);
        for tx in transactions {
            // Merge account updates so that state transitions A->B->C become A->C.
            match updated_accounts.entry(tx.account_id()) {
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
        // One thing to note:
        // This still allows transaction `A` to consume an unauthenticated note `x` and output note
        // `y` and for transaction `B` to consume an unauthenticated note `y` and output
        // note `x` (i.e., have a circular dependency between transactions).
        let mut output_notes = BatchOutputNoteTracker::new(transactions.iter().map(AsRef::as_ref))?;
        let mut input_notes = vec![];

        for tx in transactions {
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
                        if proposed_batch
                            .note_inclusion_proofs()
                            .contains_note(&input_note_header.id())
                        {
                            // authenticate_unauthenticated_note
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
        let output_notes_smt = BatchNoteTree::with_contiguous_leaves(
            output_notes.iter().map(|note| (note.id(), note.metadata())),
        )
        .expect("output note tracker should return an error for duplicate notes");

        Ok(ProvenBatch::new(
            id,
            updated_accounts,
            input_notes,
            output_notes_smt,
            output_notes,
            batch_expiration_block_num,
        ))
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
// TODO: Remove allow once used.
#[allow(dead_code)]
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
        .map_err(|_| BatchError::UnauthenticatedNoteAuthenticationFailed {
            note_id: note_header.id(),
            block_num: proof.location().block_num(),
        })
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;

    use miden_crypto::merkle::MerklePath;
    use miden_lib::{account::wallets::BasicWallet, transaction::TransactionKernel};
    use miden_objects::{
        account::{Account, AccountBuilder},
        note::{Note, NoteInclusionProof, NoteInclusionProofs},
        testing::{account_id::AccountIdBuilder, note::NoteBuilder},
        transaction::InputNote,
        BatchAccountUpdateError,
    };
    use rand::{rngs::SmallRng, SeedableRng};
    use vm_core::assert_matches;
    use vm_processor::Digest;

    use super::*;
    use crate::testing::MockProvenTxBuilder;

    fn mock_account_id(num: u8) -> AccountId {
        AccountIdBuilder::new().build_with_rng(&mut SmallRng::from_seed([num; 32]))
    }

    fn mock_wallet_account(num: u8) -> Account {
        AccountBuilder::new([num; 32])
            .with_component(BasicWallet)
            .build_existing()
            .unwrap()
    }

    pub fn mock_note(num: u8) -> Note {
        let sender = mock_account_id(num);
        NoteBuilder::new(sender, SmallRng::from_seed([num; 32]))
            .build(&TransactionKernel::assembler().with_debug_mode(true))
            .unwrap()
    }

    pub fn mock_output_note(num: u8) -> OutputNote {
        OutputNote::Full(mock_note(num))
    }

    pub fn mock_proof(node_index: u16) -> NoteInclusionProof {
        NoteInclusionProof::new(BlockNumber::from(0), node_index, MerklePath::new(vec![])).unwrap()
    }

    /// Tests that an error is returned if the same unauthenticated input note appears multiple
    /// times in different transactions.
    #[test]
    fn duplicate_unauthenticated_input_notes() -> anyhow::Result<()> {
        let account1 = mock_wallet_account(10);
        let account2 = mock_wallet_account(100);

        let note0 = mock_note(50);
        let tx1 =
            MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
                .unauthenticated_notes(vec![note0.clone()])
                .build()?;
        let tx2 =
            MockProvenTxBuilder::with_account(account2.id(), Digest::default(), account2.hash())
                .unauthenticated_notes(vec![note0.clone()])
                .build()?;

        let error = LocalBatchProver::prove(ProposedBatch::new(
            [tx1.clone(), tx2.clone()].into_iter().map(Arc::new).collect(),
            NoteInclusionProofs::default(),
        ))
        .unwrap_err();

        assert_matches!(error, BatchError::DuplicateInputNote {
            note_nullifier,
            first_transaction_id,
            second_transaction_id
          } if note_nullifier == note0.nullifier() &&
            first_transaction_id == tx1.id() &&
            second_transaction_id == tx2.id()
        );

        Ok(())
    }

    /// Tests that an error is returned if the same authenticated input note appears multiple
    /// times in different transactions.
    #[test]
    fn duplicate_authenticated_input_notes() -> anyhow::Result<()> {
        let account1 = mock_wallet_account(10);
        let account2 = mock_wallet_account(100);

        let note0 = mock_note(50);
        let tx1 =
            MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
                .authenticated_notes(vec![note0.clone()])
                .build()?;
        let tx2 =
            MockProvenTxBuilder::with_account(account2.id(), Digest::default(), account2.hash())
                .authenticated_notes(vec![note0.clone()])
                .build()?;

        let error = LocalBatchProver::prove(ProposedBatch::new(
            [tx1.clone(), tx2.clone()].into_iter().map(Arc::new).collect(),
            NoteInclusionProofs::default(),
        ))
        .unwrap_err();

        assert_matches!(error, BatchError::DuplicateInputNote {
            note_nullifier,
            first_transaction_id,
            second_transaction_id
          } if note_nullifier == note0.nullifier() &&
            first_transaction_id == tx1.id() &&
            second_transaction_id == tx2.id()
        );

        Ok(())
    }

    /// Tests that an error is returned if the same input note appears multiple times in different
    /// transactions as an unauthenticated or authenticated note.
    #[test]
    fn duplicate_mixed_input_notes() -> anyhow::Result<()> {
        let account1 = mock_wallet_account(10);
        let account2 = mock_wallet_account(100);

        let note0 = mock_note(50);
        let tx1 =
            MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
                .unauthenticated_notes(vec![note0.clone()])
                .build()?;
        let tx2 =
            MockProvenTxBuilder::with_account(account2.id(), Digest::default(), account2.hash())
                .authenticated_notes(vec![note0.clone()])
                .build()?;

        let error = LocalBatchProver::prove(ProposedBatch::new(
            [tx1.clone(), tx2.clone()].into_iter().map(Arc::new).collect(),
            NoteInclusionProofs::default(),
        ))
        .unwrap_err();

        assert_matches!(error, BatchError::DuplicateInputNote {
            note_nullifier,
            first_transaction_id,
            second_transaction_id
          } if note_nullifier == note0.nullifier() &&
            first_transaction_id == tx1.id() &&
            second_transaction_id == tx2.id()
        );

        Ok(())
    }

    /// Tests that an error is returned if the same output note appears multiple times in different
    /// transactions.
    #[test]
    fn duplicate_output_notes() -> anyhow::Result<()> {
        let account1 = mock_wallet_account(10);
        let account2 = mock_wallet_account(100);

        let note0 = mock_output_note(50);
        let tx1 =
            MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
                .output_notes(vec![note0.clone()])
                .build()?;
        let tx2 =
            MockProvenTxBuilder::with_account(account2.id(), Digest::default(), account2.hash())
                .output_notes(vec![note0.clone()])
                .build()?;

        let error = LocalBatchProver::prove(ProposedBatch::new(
            [tx1.clone(), tx2.clone()].into_iter().map(Arc::new).collect(),
            NoteInclusionProofs::default(),
        ))
        .unwrap_err();

        assert_matches!(error, BatchError::DuplicateOutputNote {
          note_id,
          first_transaction_id,
          second_transaction_id
        } if note_id == note0.id() &&
          first_transaction_id == tx1.id() &&
          second_transaction_id == tx2.id());

        Ok(())
    }

    /// Tests that a note created and consumed in the same batch are erased from the input and
    /// output note commitments.
    #[test]
    fn note_created_and_consumed_in_same_batch() -> anyhow::Result<()> {
        let account1 = mock_wallet_account(10);
        let account2 = mock_wallet_account(100);

        let note0 = mock_note(50);
        let tx1 =
            MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
                .output_notes(vec![OutputNote::Full(note0.clone())])
                .build()?;
        let tx2 =
            MockProvenTxBuilder::with_account(account2.id(), Digest::default(), account2.hash())
                .unauthenticated_notes(vec![note0.clone()])
                .build()?;

        let batch = LocalBatchProver::prove(ProposedBatch::new(
            [tx1, tx2].into_iter().map(Arc::new).collect(),
            NoteInclusionProofs::default(),
        ))?;

        assert_eq!(batch.input_notes().len(), 0);
        assert_eq!(batch.output_notes().len(), 0);
        assert_eq!(batch.output_notes_tree().num_leaves(), 0);

        Ok(())
    }

    /// Test that an authenticated input note that is also created in the same batch does not error
    /// and instead is marked as consumed.
    /// - This requires a nullifier collision on the input and output note which is very unlikely in
    ///   practice.
    /// - This makes the created note unspendable as its nullifier is added to the nullifier tree.
    /// - The batch kernel cannot return an error in this case as it can't detect this condition due
    ///   to only having the nullifier for authenticated input notes _but_ not having the nullifier
    ///   for private output notes.
    /// - We test this to ensure the kernel does something reasonable in this case and it is not an
    ///   attack vector.
    #[test]
    fn authenticated_note_created_in_same_batch() -> anyhow::Result<()> {
        let account1 = mock_wallet_account(10);
        let account2 = mock_wallet_account(100);

        let note0 = mock_note(50);
        let tx1 =
            MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
                .output_notes(vec![OutputNote::Full(note0.clone())])
                .build()?;
        let tx2 =
            MockProvenTxBuilder::with_account(account2.id(), Digest::default(), account2.hash())
                .authenticated_notes(vec![note0.clone()])
                .build()?;

        let batch = LocalBatchProver::prove(ProposedBatch::new(
            [tx1, tx2].into_iter().map(Arc::new).collect(),
            NoteInclusionProofs::default(),
        ))?;

        assert_eq!(batch.input_notes().len(), 1);
        assert_eq!(batch.output_notes().len(), 1);
        assert_eq!(batch.output_notes_tree().num_leaves(), 1);

        Ok(())
    }

    /// Test that an unauthenticated input note for which a proof exists is converted into an
    /// authenticated one and becomes part of the batch's input note commitment.
    #[test]
    fn unauthenticated_note_converted_authenticated() -> anyhow::Result<()> {
        let account1 = mock_wallet_account(10);

        let note0 = mock_note(150);
        let tx1 =
            MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
                .unauthenticated_notes(vec![note0.clone()])
                .build()?;

        let mock_note_inclusion_proofs =
            NoteInclusionProofs::new(vec![], BTreeMap::from([(note0.id(), mock_proof(0))]));

        let batch = LocalBatchProver::prove(ProposedBatch::new(
            [tx1].into_iter().map(Arc::new).collect(),
            mock_note_inclusion_proofs,
        ))?;

        // We expect the unauthenticated input note to have become an authenticated one,
        // meaning it is part of the input note commitment.
        assert_eq!(batch.input_notes().len(), 1);
        assert_eq!(batch.output_notes().len(), 0);

        Ok(())
    }

    /// Test that multiple transactions against the same account 1) can be correctly executed and
    /// 2) that an error is returned if they are incorrectly ordered.
    #[test]
    fn multiple_transactions_against_same_account() -> anyhow::Result<()> {
        let account1 = mock_wallet_account(10);

        // Use some random hash as the initial state commitment of tx1.
        let initial_state_commitment = Digest::default();
        let tx1 = MockProvenTxBuilder::with_account(
            account1.id(),
            initial_state_commitment,
            account1.hash(),
        )
        .output_notes(vec![mock_output_note(0)])
        .build()?;

        // Use some random hash as the final state commitment of tx2.
        let final_state_commitment = mock_note(10).hash();
        let tx2 = MockProvenTxBuilder::with_account(
            account1.id(),
            account1.hash(),
            final_state_commitment,
        )
        .build()?;

        // Success: Transactions are correctly ordered.
        let batch = LocalBatchProver::prove(ProposedBatch::new(
            [tx1.clone(), tx2.clone()].into_iter().map(Arc::new).collect(),
            NoteInclusionProofs::default(),
        ))?;

        assert_eq!(batch.account_updates().len(), 1);
        // Assert that the initial state commitment from tx1 is used and the final state commitment
        // from tx2.
        assert_eq!(
            batch.account_updates().get(&account1.id()).unwrap().initial_state_commitment(),
            initial_state_commitment
        );
        assert_eq!(
            batch.account_updates().get(&account1.id()).unwrap().final_state_commitment(),
            final_state_commitment
        );

        // Error: Transactions are incorrectly ordered.
        let error = LocalBatchProver::prove(ProposedBatch::new(
            [tx2.clone(), tx1.clone()].into_iter().map(Arc::new).collect(),
            NoteInclusionProofs::default(),
        ))
        .unwrap_err();

        assert_matches!(
            error,
            BatchError::AccountUpdateError {
                source: BatchAccountUpdateError::AccountUpdateInitialStateMismatch(tx_id),
                ..
            } if tx_id == tx1.id()
        );

        Ok(())
    }

    /// Tests that the input and outputs notes commitment is correctly computed.
    /// - Notes created and consumed in the same batch are erased from these commitments.
    /// - The input note commitment is sorted by the order in which the notes appeared in the batch.
    /// - The output note commitment is sorted by [`NoteId`].
    #[test]
    fn input_and_output_notes_commitment() -> anyhow::Result<()> {
        let account1 = mock_wallet_account(10);
        let account2 = mock_wallet_account(100);

        let note0 = mock_output_note(50);
        let note1 = mock_note(60);
        let note2 = mock_output_note(70);
        let note3 = mock_output_note(80);
        let note4 = mock_note(90);
        let note5 = mock_note(100);

        let tx1 =
            MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
                .unauthenticated_notes(vec![note1.clone(), note5.clone()])
                .output_notes(vec![note0.clone()])
                .build()?;
        let tx2 =
            MockProvenTxBuilder::with_account(account2.id(), Digest::default(), account2.hash())
                .unauthenticated_notes(vec![note4.clone()])
                .output_notes(vec![OutputNote::Full(note1.clone()), note2.clone(), note3.clone()])
                .build()?;

        let batch = LocalBatchProver::prove(ProposedBatch::new(
            [tx1.clone(), tx2.clone()].into_iter().map(Arc::new).collect(),
            NoteInclusionProofs::default(),
        ))?;

        // We expecte note1 to be erased from the input/output notes as it is created and consumed
        // in the batch.
        let mut expected_output_notes = [note0, note2, note3];
        // We expect a vector sorted by NoteId.
        expected_output_notes.sort_unstable_by_key(OutputNote::id);

        assert_eq!(batch.output_notes().len(), 3);
        assert_eq!(batch.output_notes(), expected_output_notes);

        assert_eq!(batch.output_notes_tree().num_leaves(), 3);

        // Input notes are sorted by the order in which they appeared in the batch.
        assert_eq!(batch.input_notes().len(), 2);
        assert_eq!(
            batch.input_notes(),
            &[
                InputNoteCommitment::from(&InputNote::unauthenticated(note5)),
                InputNoteCommitment::from(&InputNote::unauthenticated(note4)),
            ]
        );

        Ok(())
    }

    #[test]
    fn batch_expiration() -> anyhow::Result<()> {
        let account1 = mock_wallet_account(10);
        let account2 = mock_wallet_account(100);

        let tx1 =
            MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
                .expiration_block_num(BlockNumber::from(35))
                .build()?;
        let tx2 =
            MockProvenTxBuilder::with_account(account2.id(), Digest::default(), account2.hash())
                .expiration_block_num(BlockNumber::from(30))
                .build()?;

        let batch = LocalBatchProver::prove(ProposedBatch::new(
            [tx1, tx2].into_iter().map(Arc::new).collect(),
            NoteInclusionProofs::default(),
        ))?;

        assert_eq!(batch.batch_expiration_block_num(), BlockNumber::from(30));

        Ok(())
    }
}
