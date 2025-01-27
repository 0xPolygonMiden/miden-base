use alloc::{
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
    vec::Vec,
};

use miden_objects::{
    account::{AccountId, AccountUpdate},
    batch::{BatchId, BatchNoteTree},
    note::{NoteHeader, NoteId},
    transaction::{InputNoteCommitment, OutputNote, ProvenTransaction},
};

use crate::{BatchError, ProposedBatch, ProvenBatch};

pub struct LocalBatchProver {}

impl LocalBatchProver {
    pub fn prove(proposed_batch: ProposedBatch) -> Result<ProvenBatch, BatchError> {
        let transactions = proposed_batch.transactions();
        let id = BatchId::compute(transactions.iter().map(|tx| tx.id()));

        // Populate batch output notes and updated accounts.
        let mut output_notes = OutputNoteTracker::new(transactions.iter().map(|tx| tx.as_ref()))?;
        let mut updated_accounts = BTreeMap::<AccountId, AccountUpdate>::new();
        let mut unauthenticated_input_notes = BTreeSet::new();
        for tx in transactions {
            // Merge account updates so that state transitions A->B->C become A->C.
            match updated_accounts.entry(tx.account_id()) {
                Entry::Vacant(vacant) => {
                    vacant.insert(tx.account_update().clone());
                },
                Entry::Occupied(occupied) => {
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
        }

        // Populate batch produced nullifiers and match output notes with corresponding
        // unauthenticated input notes in the same batch, which are removed from the unauthenticated
        // input notes set.
        //
        // One thing to note:
        // This still allows transaction `A` to consume an unauthenticated note `x` and output note
        // `y` and for transaction `B` to consume an unauthenticated note `y` and output
        // note `x` (i.e., have a circular dependency between transactions), but this is not
        // a problem.
        let mut input_notes = vec![];
        for tx in transactions {
            for input_note in tx.input_notes().iter() {
                // Header is presented only for unauthenticated input notes.
                let input_note = match input_note.header() {
                    Some(input_note_header) => {
                        if output_notes.remove_note(input_note_header)? {
                            continue;
                        }

                        // If an unauthenticated note was found in the store, transform it to an
                        // authenticated one (i.e. erase additional note details
                        // except the nullifier)
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
        .expect("Unreachable: fails only if the output note list contains duplicates");

        Ok(ProvenBatch::new(
            id,
            updated_accounts,
            input_notes,
            output_notes_smt,
            output_notes,
        ))
    }
}

#[derive(Debug)]
struct OutputNoteTracker {
    output_notes: Vec<Option<OutputNote>>,
    output_note_index: BTreeMap<NoteId, usize>,
}

impl OutputNoteTracker {
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
