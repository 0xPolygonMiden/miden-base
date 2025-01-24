use alloc::{sync::Arc, vec::Vec};

use miden_objects::transaction::ProvenTransaction;

// TODO: Document.
#[derive(Debug, Clone)]
pub struct ProposedBatch {
    transactions: Vec<Arc<ProvenTransaction>>,
}

impl ProposedBatch {
    pub fn new(transactions: Vec<Arc<ProvenTransaction>>) -> Self {
        Self { transactions }
    }
}
