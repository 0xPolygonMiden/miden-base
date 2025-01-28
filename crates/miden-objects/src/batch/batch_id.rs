use alloc::vec::Vec;
use core::borrow::Borrow;

use miden_crypto::hash::Digest;
use vm_core::crypto::hash::Blake3Digest;
use vm_processor::crypto::Blake3_256;

use crate::transaction::TransactionId;

// BATCH ID
// ================================================================================================

/// Uniquely identifies a [`ProvenBatch`](crate::batch::ProvenBatch).
// TODO: Document how this is computed.
// TODO: Should this really be a Blake3 hash? We have to compute this in the block kernel
// eventually, so we'd probably want RPO instead?
// TODO: Compute batch ID as hash over tx ID _and_ account ID.
#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct BatchId(Blake3Digest<32>);

impl BatchId {
    /// Calculates a batch ID from the given set of transactions.
    pub fn compute<T>(txs: impl Iterator<Item = T>) -> Self
    where
        T: Borrow<TransactionId>,
    {
        let mut buf = Vec::with_capacity(32 * txs.size_hint().0);
        for tx in txs {
            buf.extend_from_slice(&tx.borrow().as_bytes());
        }
        Self(Blake3_256::hash(&buf))
    }
}

impl core::fmt::Display for BatchId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&miden_crypto::utils::bytes_to_hex_string(self.0.as_bytes()))
    }
}
