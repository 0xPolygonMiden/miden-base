use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};

use crate::{
    block::BlockHeader,
    note::{NoteId, NoteInclusionProof},
    transaction::{ChainMmr, ProvenTransaction},
};

/// A proposed batch of transactions with all necessary data to validate it.
#[derive(Debug, Clone)]
pub struct ProposedBatch {
    transactions: Vec<Arc<ProvenTransaction>>,
    block_header: BlockHeader,
    /// The chain MMR used to authenticate:
    /// - all unauthenticated notes that can be authenticated,
    /// - all block hashes referenced by the transactions in the batch.
    block_chain: ChainMmr,
    /// The note inclusion proofs for unauthenticated notes that were consumed in the batch which
    /// can be authenticated.
    authenticatable_unauthenticated_notes: BTreeMap<NoteId, NoteInclusionProof>,
}

impl ProposedBatch {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`ProposedBatch`] from the provided parts.
    pub fn new(
        transactions: Vec<Arc<ProvenTransaction>>,
        block_header: BlockHeader,
        block_chain: ChainMmr,
        authenticatable_unauthenticated_notes: BTreeMap<NoteId, NoteInclusionProof>,
    ) -> Self {
        Self {
            transactions,
            block_header,
            block_chain,
            authenticatable_unauthenticated_notes,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a slice of the [`ProvenTransaction`]s in the batch.
    pub fn transactions(&self) -> &[Arc<ProvenTransaction>] {
        &self.transactions
    }

    /// Returns the note inclusion proofs for unauthenticated notes that were consumed in the batch
    /// which can be authenticated.
    pub fn note_inclusion_proofs(&self) -> &BTreeMap<NoteId, NoteInclusionProof> {
        &self.authenticatable_unauthenticated_notes
    }

    pub fn into_parts(
        self,
    ) -> (
        Vec<Arc<ProvenTransaction>>,
        BlockHeader,
        ChainMmr,
        BTreeMap<NoteId, NoteInclusionProof>,
    ) {
        (
            self.transactions,
            self.block_header,
            self.block_chain,
            self.authenticatable_unauthenticated_notes,
        )
    }
}
