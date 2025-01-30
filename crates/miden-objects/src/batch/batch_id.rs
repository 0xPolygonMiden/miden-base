use alloc::{string::String, vec::Vec};

use vm_core::{Felt, ZERO};
use vm_processor::Digest;

use crate::{transaction::ProvenTransaction, Hasher};

// BATCH ID
// ================================================================================================

/// Uniquely identifies a batch of transactions, i.e. both
/// [`ProposedBatch`](crate::batch::ProposedBatch) and [`ProvenBatch`](crate::batch::ProvenBatch).
///
/// This is a sequential hash of the tuple `(TRANSACTION_ID || [account_id_prefix,
/// account_id_suffix, 0, 0])` of all transactions and the accounts their executed against in the
/// batch.
#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct BatchId(Digest);

impl BatchId {
    /// Calculates a batch ID from the given set of transactions.
    pub fn compute<'tx, T>(txs: T) -> Self
    where
        T: Iterator<Item = &'tx ProvenTransaction>,
    {
        let mut elements: Vec<Felt> = Vec::new();
        for tx in txs {
            elements.extend_from_slice(tx.id().as_elements());
            let [account_id_prefix, account_id_suffix] = <[Felt; 2]>::from(tx.account_id());
            elements.extend_from_slice(&[account_id_prefix, account_id_suffix, ZERO, ZERO]);
        }

        Self(Hasher::hash_elements(&elements))
    }

    /// Returns the elements representation of this batch ID.
    pub fn as_elements(&self) -> &[Felt] {
        self.0.as_elements()
    }

    /// Returns the byte representation of this batch ID.
    pub fn as_bytes(&self) -> [u8; 32] {
        self.0.as_bytes()
    }

    /// Returns a big-endian, hex-encoded string.
    pub fn to_hex(&self) -> String {
        self.0.to_hex()
    }
}

impl core::fmt::Display for BatchId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}
