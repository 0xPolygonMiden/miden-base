use alloc::{collections::BTreeMap, vec::Vec};

use miden_crypto::merkle::MerkleError;
use vm_processor::Digest;

use crate::{
    batch::{BatchId, ProvenBatch},
    block::BlockNumber,
    errors::ProposedBatchError,
    note::{NoteHeader, NoteId, NoteInclusionProof, Nullifier},
    transaction::{ChainMmr, InputNoteCommitment, OutputNote, ProvenTransaction, TransactionId},
    ProposedBlockError,
};

// NOTE ERASER
// ================================================================================================

/// A helper struct to track input and output notes and erase those that are created and consumed
/// within the same batch or block.
///
/// Its main purpose is to check for duplicates and allow for removal of output notes that are
/// consumed in the same batch, so are not output notes of the batch.
///
/// The approach for this is that the output note set is initialized to the union of all output
/// notes of the transactions in the batch.
/// Then (outside of this struct) all input notes of transactions in the batch which are also output
/// notes can be removed, as they are considered consumed within the batch and will not be visible
/// as created or consumed notes for the batch.
#[derive(Debug)]
pub(crate) struct InputOutputNoteTracker<ContainerId> {
    /// An index from Nullifier to the identifier that consumes it (either a [`TransactionId`] or
    /// [`BatchId`](crate::batch::BatchId)).
    input_notes: BTreeMap<Nullifier, (ContainerId, InputNoteCommitment)>,
    /// An index from [`NoteId`]s to the transaction that creates the note and the note itself.
    /// The transaction ID is tracked to produce better errors when a duplicate note is
    /// encountered.
    output_notes: BTreeMap<NoteId, (ContainerId, OutputNote)>,
}

impl InputOutputNoteTracker<TransactionId> {
    /// TODO
    pub fn from_transactions<'a>(
        txs: impl Iterator<Item = &'a ProvenTransaction> + Clone,
        unauthenticated_note_proofs: &BTreeMap<NoteId, NoteInclusionProof>,
        chain_mmr: &ChainMmr,
    ) -> Result<(Vec<InputNoteCommitment>, Vec<OutputNote>), ProposedBatchError> {
        let input_notes_iter = txs.clone().flat_map(|tx| {
            tx.input_notes()
                .iter()
                .map(|input_note_commitment| (input_note_commitment.clone(), tx.id()))
        });
        let output_notes_iter = txs.flat_map(|tx| {
            tx.output_notes().iter().map(|output_note| (output_note.clone(), tx.id()))
        });

        let mut tracker = Self::from_iter(
            input_notes_iter,
            output_notes_iter,
            unauthenticated_note_proofs,
            chain_mmr,
        )
        .map_err(ProposedBatchError::from)?;

        let batch_input_notes = tracker.erase_notes().map_err(ProposedBatchError::from)?;

        Ok((batch_input_notes, tracker.into_final_output_notes()))
    }
}

impl InputOutputNoteTracker<BatchId> {
    /// TODO
    #[allow(clippy::type_complexity)]
    pub fn from_batches<'a>(
        batches: impl Iterator<Item = &'a ProvenBatch> + Clone,
        unauthenticated_note_proofs: &BTreeMap<NoteId, NoteInclusionProof>,
        chain_mmr: &ChainMmr,
    ) -> Result<
        (Vec<InputNoteCommitment>, BTreeMap<NoteId, (BatchId, OutputNote)>),
        ProposedBlockError,
    > {
        let input_notes_iter = batches.clone().flat_map(|batch| {
            batch
                .input_notes()
                .iter()
                .map(|input_note_commitment| (input_note_commitment.clone(), batch.id()))
        });

        let output_notes_iter = batches.flat_map(|batch| {
            batch.output_notes().iter().map(|output_note| (output_note.clone(), batch.id()))
        });

        let mut tracker = Self::from_iter(
            input_notes_iter,
            output_notes_iter,
            unauthenticated_note_proofs,
            chain_mmr,
        )
        .map_err(ProposedBlockError::from)?;

        let block_input_notes = tracker.erase_notes().map_err(ProposedBlockError::from)?;

        Ok((block_input_notes, tracker.output_notes))
    }
}

impl<ContainerId: Copy> InputOutputNoteTracker<ContainerId> {
    fn from_iter(
        input_notes_iter: impl Iterator<Item = (InputNoteCommitment, ContainerId)>,
        output_notes_iter: impl Iterator<Item = (OutputNote, ContainerId)>,
        unauthenticated_note_proofs: &BTreeMap<NoteId, NoteInclusionProof>,
        chain_mmr: &ChainMmr,
    ) -> Result<Self, NoteEraserError<ContainerId>> {
        let mut input_notes = BTreeMap::new();
        let mut output_notes = BTreeMap::new();

        for (input_note_commitment, container_id) in input_notes_iter {
            let input_note_commitment = if let Some(note_header) = input_note_commitment.header() {
                if let Some(proof) = unauthenticated_note_proofs.get(&note_header.id()) {
                    // Transform unauthenticated notes into authenticated ones if the provided proof
                    // is valid.
                    Self::authenticate_unauthenticated_note(
                        input_note_commitment.nullifier(),
                        note_header,
                        proof,
                        chain_mmr,
                    )?
                } else {
                    input_note_commitment
                }
            } else {
                input_note_commitment
            };

            let nullifier = input_note_commitment.nullifier();
            if let Some((first_container_id, _)) =
                input_notes.insert(nullifier, (container_id, input_note_commitment))
            {
                return Err(NoteEraserError::DuplicateInputNote {
                    note_nullifier: nullifier,
                    first_container_id,
                    second_container_id: container_id,
                });
            }
        }

        for (note, container_id) in output_notes_iter {
            if let Some((first_container_id, _)) =
                output_notes.insert(note.id(), (container_id, note.clone()))
            {
                return Err(NoteEraserError::DuplicateOutputNote {
                    note_id: note.id(),
                    first_container_id,
                    second_container_id: container_id,
                });
            }
        }

        Ok(Self { input_notes, output_notes })
    }

    fn erase_notes(&mut self) -> Result<Vec<InputNoteCommitment>, NoteEraserError<ContainerId>> {
        let mut final_input_notes = Vec::new();

        for (_, input_note_commitment) in self.input_notes.values() {
            match input_note_commitment.header() {
                Some(input_note_header) => {
                    // If the unauthenticated note is created as an output note we erase it by not
                    // adding it to the final_input_notes.
                    if !Self::remove_note(input_note_header, &mut self.output_notes)? {
                        final_input_notes.push(input_note_commitment.clone());
                    }
                },
                None => {
                    final_input_notes.push(input_note_commitment.clone());
                },
            }
        }

        Ok(final_input_notes)
    }

    /// Should be called after [`Self::erase_notes`] to collect the remaining (non-erased) output
    /// notes into the final set of output notes.
    fn into_final_output_notes(self) -> Vec<OutputNote> {
        self.output_notes.into_iter().map(|(_, (_, output_note))| output_note).collect()
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
    fn remove_note(
        input_note_header: &NoteHeader,
        output_notes: &mut BTreeMap<NoteId, (ContainerId, OutputNote)>, // output
    ) -> Result<bool, NoteEraserError<ContainerId>> {
        let id = input_note_header.id();
        if let Some((_, output_note)) = output_notes.remove(&id) {
            // Check if the notes with the same ID have differing hashes.
            // This could happen if the metadata of the notes is different, which we consider an
            // error.
            let input_hash = input_note_header.hash();
            let output_hash = output_note.hash();
            if output_hash != input_hash {
                return Err(NoteEraserError::NoteHashesMismatch { id, input_hash, output_hash });
            }

            return Ok(true);
        }

        Ok(false)
    }

    /// Verifies the note inclusion proof for the given input note commitment parts (nullifier and
    /// note header). Uses the block header referenced by the inclusion proof from the chain MMR.
    ///
    /// If the proof is valid, it means the note is part of the chain and it is "marked" as
    /// authenticated by returning an [`InputNoteCommitment`] without the note header.
    fn authenticate_unauthenticated_note(
        nullifier: Nullifier,
        note_header: &NoteHeader,
        proof: &NoteInclusionProof,
        chain_mmr: &ChainMmr,
    ) -> Result<InputNoteCommitment, NoteEraserError<ContainerId>> {
        let note_block_header =
            chain_mmr.get_block(proof.location().block_num()).ok_or_else(|| {
                NoteEraserError::UnauthenticatedInputNoteBlockNotInChainMmr {
                    block_number: proof.location().block_num(),
                    note_id: note_header.id(),
                }
            })?;

        let note_index = proof.location().node_index_in_block().into();
        let note_hash = note_header.hash();
        proof
            .note_path()
            .verify(note_index, note_hash, &note_block_header.note_root())
            .map_err(|source| NoteEraserError::UnauthenticatedNoteAuthenticationFailed {
                note_id: note_header.id(),
                block_num: proof.location().block_num(),
                source,
            })?;

        // Erase the note header from the input note.
        Ok(InputNoteCommitment::from(nullifier))
    }
}

enum NoteEraserError<ContainerId: Copy> {
    DuplicateInputNote {
        note_nullifier: Nullifier,
        first_container_id: ContainerId,
        second_container_id: ContainerId,
    },
    DuplicateOutputNote {
        note_id: NoteId,
        first_container_id: ContainerId,
        second_container_id: ContainerId,
    },
    NoteHashesMismatch {
        id: NoteId,
        input_hash: Digest,
        output_hash: Digest,
    },
    UnauthenticatedInputNoteBlockNotInChainMmr {
        block_number: BlockNumber,
        note_id: NoteId,
    },
    UnauthenticatedNoteAuthenticationFailed {
        note_id: NoteId,
        block_num: BlockNumber,
        source: MerkleError,
    },
}

impl From<NoteEraserError<BatchId>> for ProposedBlockError {
    fn from(error: NoteEraserError<BatchId>) -> Self {
        match error {
            NoteEraserError::DuplicateInputNote {
                note_nullifier,
                first_container_id,
                second_container_id,
            } => ProposedBlockError::DuplicateInputNote {
                note_nullifier,
                first_batch_id: first_container_id,
                second_batch_id: second_container_id,
            },
            NoteEraserError::DuplicateOutputNote {
                note_id,
                first_container_id,
                second_container_id,
            } => ProposedBlockError::DuplicateOutputNote {
                note_id,
                first_batch_id: first_container_id,
                second_batch_id: second_container_id,
            },
            NoteEraserError::NoteHashesMismatch { id, input_hash, output_hash } => {
                ProposedBlockError::NoteHashesMismatch { id, input_hash, output_hash }
            },
            NoteEraserError::UnauthenticatedInputNoteBlockNotInChainMmr {
                block_number,
                note_id,
            } => ProposedBlockError::UnauthenticatedInputNoteBlockNotInChainMmr {
                block_number,
                note_id,
            },
            NoteEraserError::UnauthenticatedNoteAuthenticationFailed {
                note_id,
                block_num,
                source,
            } => ProposedBlockError::UnauthenticatedNoteAuthenticationFailed {
                note_id,
                block_num,
                source,
            },
        }
    }
}

impl From<NoteEraserError<TransactionId>> for ProposedBatchError {
    fn from(error: NoteEraserError<TransactionId>) -> Self {
        match error {
            NoteEraserError::DuplicateInputNote {
                note_nullifier,
                first_container_id,
                second_container_id,
            } => ProposedBatchError::DuplicateInputNote {
                note_nullifier,
                first_transaction_id: first_container_id,
                second_transaction_id: second_container_id,
            },
            NoteEraserError::DuplicateOutputNote {
                note_id,
                first_container_id,
                second_container_id,
            } => ProposedBatchError::DuplicateOutputNote {
                note_id,
                first_transaction_id: first_container_id,
                second_transaction_id: second_container_id,
            },
            NoteEraserError::NoteHashesMismatch { id, input_hash, output_hash } => {
                ProposedBatchError::NoteHashesMismatch { id, input_hash, output_hash }
            },
            NoteEraserError::UnauthenticatedInputNoteBlockNotInChainMmr {
                block_number,
                note_id,
            } => ProposedBatchError::UnauthenticatedInputNoteBlockNotInChainMmr {
                block_number,
                note_id,
            },
            NoteEraserError::UnauthenticatedNoteAuthenticationFailed {
                note_id,
                block_num,
                source,
            } => ProposedBatchError::UnauthenticatedNoteAuthenticationFailed {
                note_id,
                block_num,
                source,
            },
        }
    }
}
