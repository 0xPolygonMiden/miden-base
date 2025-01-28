use alloc::{sync::Arc, vec::Vec};

use crate::{note::NoteInclusionProofs, transaction::ProvenTransaction};

/// A proposed batch of transactions with all necessary data to validate it.
#[derive(Debug, Clone)]
pub struct ProposedBatch {
    transactions: Vec<Arc<ProvenTransaction>>,
    /// The note inclusion proofs for unauthenticated notes that were consumed in the batch which
    /// can be authenticated.
    authenticatable_unauthenticated_notes: NoteInclusionProofs,
}

impl ProposedBatch {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`ProposedBatch`] from the provided parts.
    pub fn new(
        transactions: Vec<Arc<ProvenTransaction>>,
        authenticatable_unauthenticated_notes: NoteInclusionProofs,
    ) -> Self {
        Self {
            transactions,
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
    pub fn note_inclusion_proofs(&self) -> &NoteInclusionProofs {
        &self.authenticatable_unauthenticated_notes
    }
}
