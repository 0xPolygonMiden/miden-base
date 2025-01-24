use alloc::{sync::Arc, vec::Vec};

use miden_objects::{note::NoteAuthenticationInfo, transaction::ProvenTransaction};

// TODO: Document.
#[derive(Debug, Clone)]
pub struct ProposedBatch {
    transactions: Vec<Arc<ProvenTransaction>>,
    /// This is used to transform unauthenticated notes into authenticated ones. Unauthenticated
    /// notes can be consumed by transactions but the notes are in fact part of the chain.
    stored_unauthenticated_notes: NoteAuthenticationInfo,
}

impl ProposedBatch {
    pub fn new(
        transactions: Vec<Arc<ProvenTransaction>>,
        stored_unauthenticated_notes: NoteAuthenticationInfo,
    ) -> Self {
        Self {
            transactions,
            stored_unauthenticated_notes,
        }
    }

    pub fn transactions(&self) -> &[Arc<ProvenTransaction>] {
        &self.transactions
    }

    pub fn stored_unauthenticated_notes(&self) -> &NoteAuthenticationInfo {
        &self.stored_unauthenticated_notes
    }
}
