use alloc::vec::Vec;

use crate::{
    Digest,
    block::BlockHeader,
    transaction::TransactionHeader,
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
};

// ORDERED TRANSACTION HEADERS
// ================================================================================================

/// The ordered set of transaction headers in a [`ProvenBatch`](crate::batch::ProvenBatch) or
/// [`ProvenBlock`](crate::block::ProvenBlock).
///
/// This is a newtype wrapper representing either:
/// - the set of transactions in a **batch**,
/// - or the flattened sets of transactions of each proven batch in a **block**.
///
/// This type cannot be constructed directly, but can be retrieved through:
/// - [`ProposedBatch::transaction_headers`](crate::batch::ProposedBatch::transaction_headers),
/// - [`OrderedBatches::into_transactions`](crate::batch::OrderedBatches::into_transactions).
///
/// The rationale for this requirement is that it allows a client to cheaply validate the
/// correctness of the transactions in a proven block returned by a remote prover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedTransactionHeaders(Vec<TransactionHeader>);

impl OrderedTransactionHeaders {
    /// Creates a new set of ordered transaction headers from the provided vector.
    ///
    /// # Warning
    ///
    /// See the type-level documentation for the requirements of the passed transactions.
    pub fn new_unchecked(transactions: Vec<TransactionHeader>) -> Self {
        Self(transactions)
    }

    /// Computes a commitment to the provided list of transactions.
    ///
    /// See [`BlockHeader::compute_tx_commitment`] for details.
    pub fn commitment(&self) -> Digest {
        BlockHeader::compute_tx_commitment(
            self.0.as_slice().iter().map(|tx| (tx.id(), tx.account_id())),
        )
    }

    /// Returns a reference to the underlying transaction headers.
    pub fn as_slice(&self) -> &[TransactionHeader] {
        &self.0
    }

    /// Consumes self and returns the underlying vector of transaction headers.
    pub fn into_vec(self) -> Vec<TransactionHeader> {
        self.0
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for OrderedTransactionHeaders {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.0.write_into(target)
    }
}

impl Deserializable for OrderedTransactionHeaders {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        source.read().map(OrderedTransactionHeaders::new_unchecked)
    }
}
