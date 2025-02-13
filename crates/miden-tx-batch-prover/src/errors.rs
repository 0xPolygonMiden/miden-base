use alloc::{boxed::Box, string::String};
use core::error::Error as CoreError;

use miden_objects::transaction::TransactionId;
use miden_tx::TransactionVerifierError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BatchProveError {
    #[error("failed to verify transaction {transaction_id} in transaction batch")]
    TransactionVerificationFailed {
        transaction_id: TransactionId,
        source: TransactionVerifierError,
    },
    /// Custom error variant for errors not covered by the other variants.
    #[error("{error_msg}")]
    Other {
        error_msg: Box<str>,
        // thiserror will return this when calling Error::source on DataStoreError.
        source: Option<Box<dyn CoreError + Send + Sync + 'static>>,
    },
}

impl BatchProveError {
    /// Creates a custom error using the [`BatchProveError::Other`] variant from an error
    /// message.
    pub fn other(message: impl Into<String>) -> Self {
        let message: String = message.into();
        Self::Other { error_msg: message.into(), source: None }
    }

    /// Creates a custom error using the [`BatchProveError::Other`] variant from an error
    /// message and a source error.
    pub fn other_with_source(
        message: impl Into<String>,
        source: impl CoreError + Send + Sync + 'static,
    ) -> Self {
        let message: String = message.into();
        Self::Other {
            error_msg: message.into(),
            source: Some(Box::new(source)),
        }
    }
}
