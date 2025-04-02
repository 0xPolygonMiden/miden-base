use alloc::vec::Vec;

use vm_core::utils::{ByteReader, Deserializable};
use vm_processor::DeserializationError;

use crate::{
    batch::ProvenBatch,
    transaction::OrderedTransactionHeaders,
    utils::{ByteWriter, Serializable},
};

// ORDERED BATCHES
// ================================================================================================

/// The ordered set of batches in a [`ProposedBlock`](crate::block::ProposedBlock).
///
/// This is a newtype wrapper represeting the set of batches in a proposed block. It can only be
/// retrieved from a proposed block. This type exists only to encapsulate the conversion to
/// [`OrderedTransactionHeaders`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedBatches(Vec<ProvenBatch>);

impl OrderedBatches {
    /// Creates a new set of ordered batches from the provided vector.
    pub(crate) fn new(batches: Vec<ProvenBatch>) -> Self {
        Self(batches)
    }

    /// Returns a reference to the underlying proven batches.
    pub fn as_slice(&self) -> &[ProvenBatch] {
        &self.0
    }

    /// Converts the transactions in batches into ordered transaction headers.
    pub fn to_transactions(&self) -> OrderedTransactionHeaders {
        OrderedTransactionHeaders::new(
            self.0
                .iter()
                .flat_map(|batch| batch.transactions().as_slice().iter())
                .cloned()
                .collect(),
        )
    }

    /// Consumes self and converts the transactions in batches into ordered transaction headers.
    pub fn into_transactions(self) -> OrderedTransactionHeaders {
        OrderedTransactionHeaders::new(
            self.0
                .into_iter()
                .flat_map(|batch| batch.into_transactions().into_vec().into_iter())
                .collect(),
        )
    }

    /// Consumes self and returns the underlying vector of batches.
    pub fn into_vec(self) -> Vec<ProvenBatch> {
        self.0
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for OrderedBatches {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.0.write_into(target)
    }
}

impl Deserializable for OrderedBatches {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        source.read().map(OrderedBatches::new)
    }
}
