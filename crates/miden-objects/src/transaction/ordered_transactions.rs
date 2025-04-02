use alloc::vec::Vec;

use vm_core::utils::{ByteReader, Deserializable};
use vm_processor::DeserializationError;

use crate::{
    transaction::TransactionHeader,
    utils::{ByteWriter, Serializable},
};

// ORDERED TRANSACTION HEADERS
// ================================================================================================

/// The ordered set of transaction headers in a [`ProvenBlock`](crate::block::ProvenBlock).
///
/// This is a newtype wrapper represeting the flattened sets of transactions of each proven batch in
/// a block. This requirement is not enforced by this type. It cannot be constructed directly and
/// can instead be retrieved through
/// [`OrderedBatches::into_transactions`](crate::batch::OrderedBatches::into_transactions).
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
    pub(crate) fn new(transactions: Vec<TransactionHeader>) -> Self {
        Self(transactions)
    }

    /// Creates a new set of ordered transaction headers from the provided vector.
    ///
    /// # Warning
    ///
    /// See the type-level documentation for the requirements of the passed transactions. This
    /// method is exposed only for testing purposes.
    #[cfg(feature = "testing")]
    pub fn new_unchecked(transactions: Vec<TransactionHeader>) -> Self {
        Self(transactions)
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
        source.read().map(OrderedTransactionHeaders::new)
    }
}
