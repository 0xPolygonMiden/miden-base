use alloc::{
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
    vec::Vec,
};

use miden_objects::{
    account::AccountId,
    batch::{BatchAccountUpdate, BatchId, BatchNoteTree},
    block::BlockNumber,
    note::{NoteHeader, NoteId},
    transaction::{InputNoteCommitment, OutputNote, ProvenTransaction},
};

use crate::{BatchError, ProposedBatch, ProvenBatch};

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

        // Populate batch output notes and updated accounts.
        let mut updated_accounts = BTreeMap::<AccountId, BatchAccountUpdate>::new();
        let mut unauthenticated_input_notes = BTreeSet::new();
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

            // Check unauthenticated input notes for duplicates:
            for note in tx.get_unauthenticated_notes() {
                let id = note.id();
                if !unauthenticated_input_notes.insert(id) {
                    return Err(BatchError::DuplicateUnauthenticatedNote(id));
                }
            }

            // The expiration block of the batch is the minimum of all transaction's expiration
            // block.
            batch_expiration_block_num = batch_expiration_block_num.min(tx.expiration_block_num());
        }

        // Remove all output notes from the batch output note set that are consumed by transactions.
        //
        // One thing to note:
        // This still allows transaction `A` to consume an unauthenticated note `x` and output note
        // `y` and for transaction `B` to consume an unauthenticated note `y` and output
        // note `x` (i.e., have a circular dependency between transactions), but this is not
        // a problem.
        let mut output_notes = BatchOutputNoteTracker::new(transactions.iter().map(AsRef::as_ref))?;
        let mut input_notes = vec![];

        for tx in transactions {
            for input_note in tx.input_notes().iter() {
                // Header is present only for unauthenticated input notes.
                let input_note = match input_note.header() {
                    Some(input_note_header) => {
                        // If a transaction consumes a note that is also created in this batch, the
                        // note is effectively erased from the overall output note set of the batch.
                        if output_notes.remove_note(input_note_header)? {
                            continue;
                        }

                        // If an inclusion proof for an unauthenticated note is provided and the
                        // proof is valid, it means the note is part of the chain and we can mark it
                        // as authenticated by erasing the note header.
                        if proposed_batch
                            .note_inclusion_proofs()
                            .contains_note(&input_note_header.id())
                        {
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

#[derive(Debug)]
struct BatchOutputNoteTracker {
    output_notes: Vec<Option<OutputNote>>,
    output_note_index: BTreeMap<NoteId, usize>,
}

impl BatchOutputNoteTracker {
    fn new<'a>(txs: impl Iterator<Item = &'a ProvenTransaction>) -> Result<Self, BatchError> {
        let mut output_notes = vec![];
        let mut output_note_index = BTreeMap::new();
        for tx in txs {
            for note in tx.output_notes().iter() {
                if output_note_index.insert(note.id(), output_notes.len()).is_some() {
                    return Err(BatchError::DuplicateOutputNote(note.id()));
                }
                output_notes.push(Some(note.clone()));
            }
        }

        Ok(Self { output_notes, output_note_index })
    }

    pub fn remove_note(&mut self, input_note_header: &NoteHeader) -> Result<bool, BatchError> {
        let id = input_note_header.id();
        if let Some(note_index) = self.output_note_index.remove(&id) {
            if let Some(output_note) = core::mem::take(&mut self.output_notes[note_index]) {
                let input_hash = input_note_header.hash();
                let output_hash = output_note.hash();
                if output_hash != input_hash {
                    return Err(BatchError::NoteHashesMismatch { id, input_hash, output_hash });
                }

                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn into_notes(self) -> Vec<OutputNote> {
        self.output_notes.into_iter().flatten().collect()
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;

    use miden_lib::{account::wallets::BasicWallet, transaction::TransactionKernel};
    use miden_objects::{
        account::{Account, AccountBuilder},
        note::{Note, NoteInclusionProofs},
        testing::{account_id::AccountIdBuilder, note::NoteBuilder},
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
            [tx1, tx2].into_iter().map(Arc::new).collect(),
            NoteInclusionProofs::default(),
        ))
        .unwrap_err();

        assert_matches!(error, BatchError::DuplicateUnauthenticatedNote(id) if id == note0.id());

        Ok(())
    }

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
            [tx1, tx2].into_iter().map(Arc::new).collect(),
            NoteInclusionProofs::default(),
        ))
        .unwrap_err();

        assert_matches!(error, BatchError::DuplicateOutputNote(id) if id == note0.id());

        Ok(())
    }

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

        assert_eq!(batch.account_updates().len(), 2);
        assert_eq!(batch.input_notes().len(), 0);
        assert_eq!(batch.output_notes().len(), 0);
        assert_eq!(batch.output_notes_tree().num_leaves(), 0);
        assert_eq!(
            batch.account_updates().get(&account1.id()).unwrap().final_state_commitment(),
            account1.hash()
        );
        assert_eq!(
            batch.account_updates().get(&account2.id()).unwrap().final_state_commitment(),
            account2.hash()
        );

        Ok(())
    }

    #[test]
    fn multiple_transactions_against_same_account() -> anyhow::Result<()> {
        let account1 = mock_wallet_account(10);

        let tx1 =
            MockProvenTxBuilder::with_account(account1.id(), Digest::default(), account1.hash())
                .output_notes(vec![mock_output_note(0)])
                .build()?;
        // Use some other hash as the final state commitment of tx2.
        let final_state_commitment = mock_note(10).hash();
        let tx2 = MockProvenTxBuilder::with_account(
            account1.id(),
            account1.hash(),
            final_state_commitment,
        )
        .build()?;

        // Success: Transactions are correctly ordered.
        LocalBatchProver::prove(ProposedBatch::new(
            [tx1.clone(), tx2.clone()].into_iter().map(Arc::new).collect(),
            NoteInclusionProofs::default(),
        ))?;

        // Error: Transactions are incorrectly ordered.
        let error = LocalBatchProver::prove(ProposedBatch::new(
            [tx2, tx1].into_iter().map(Arc::new).collect(),
            NoteInclusionProofs::default(),
        ))
        .unwrap_err();

        assert_matches!(error, BatchError::AccountUpdateError { .. });

        Ok(())
    }
}
