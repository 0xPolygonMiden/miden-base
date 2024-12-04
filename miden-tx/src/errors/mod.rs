use alloc::{boxed::Box, string::String};
use core::error::Error;

use miden_objects::{
    accounts::AccountId, notes::NoteId, AccountError, Felt, ProvenTransactionError,
    TransactionInputError, TransactionOutputError,
};
use miden_verifier::VerificationError;
use thiserror::Error;
use vm_processor::ExecutionError;

// TRANSACTION EXECUTOR ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum TransactionExecutorError {
    // TODO: Turn into source error after upgrading to latest miden-vm.
    #[error("failed to execute transaction kernel program: {0}")]
    TransactionProgramExecutionFailed(ExecutionError),
    #[error("failed to fetch transaction inputs from the data store")]
    FetchTransactionInputsFailed(#[source] DataStoreError),
    #[error("input account id {input_id} does not match output account id {output_id}")]
    InconsistentAccountId {
        input_id: AccountId,
        output_id: AccountId,
    },
    #[error("expected account nonce {expected:?}, found {actual:?}")]
    InconsistentAccountNonceDelta {
        expected: Option<Felt>,
        actual: Option<Felt>,
    },
    #[error("failed to construct transaction outputs")]
    TransactionOutputConstructionFailed(#[source] TransactionOutputError),
    #[error("failed to create transaction host")]
    TransactionHostCreationFailed(#[source] TransactionHostError),
}

// TRANSACTION PROVER ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum TransactionProverError {
    #[error("failed to apply account delta")]
    AccountDeltaApplyFailed(#[source] AccountError),
    #[error("failed to construct transaction outputs")]
    TransactionOutputConstructionFailed(#[source] TransactionOutputError),
    #[error("failed to build proven transaction")]
    ProvenTransactionBuildFailed(#[source] ProvenTransactionError),
    // TODO: Turn into source error after upgrading to latest miden-vm.
    #[error("failed to execute transaction kernel program: {0}")]
    TransactionProgramExecutionFailed(ExecutionError),
    #[error("failed to create transaction host")]
    TransactionHostCreationFailed(#[source] TransactionHostError),
    /// Custom error variant for errors not covered by the other variants.
    #[error("{error_msg}")]
    Custom {
        error_msg: Box<str>,
        // thiserror will return this when calling Error::source on DataStoreError.
        source: Option<Box<dyn Error + Send + Sync + 'static>>,
    },
}

impl TransactionProverError {
    /// Creates a custom error from an error message.
    pub fn custom(message: impl Into<String>) -> Self {
        let message: String = message.into();
        Self::Custom { error_msg: message.into(), source: None }
    }

    /// Creates a custom error from an error message and a source error.
    pub fn custom_with_source(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        let message: String = message.into();
        Self::Custom {
            error_msg: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

// TRANSACTION VERIFIER ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum TransactionVerifierError {
    // TODO: Turn into source error after upgrading to latest miden-vm.
    #[error("failed to verify transaction: {0}")]
    TransactionVerificationFailed(VerificationError),
    #[error(
        "transaction proof security level is {actual} but must be at least {expected_minimum}"
    )]
    InsufficientProofSecurityLevel { actual: u32, expected_minimum: u32 },
}

// TRANSACTION HOST ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum TransactionHostError {
    #[error("{0}")]
    AccountProcedureIndexMapError(String),
    #[error("failed to create account procedure info")]
    AccountProcedureInfoCreationFailed(#[source] AccountError),
}

// DATA STORE ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum DataStoreError {
    #[error("account with id {0} not found in data store")]
    AccountNotFound(AccountId),
    #[error("block with number {0} not found in data store")]
    BlockNotFound(u32),
    #[error("failed to create transaction inputs")]
    InvalidTransactionInput(#[source] TransactionInputError),
    #[error("note with id {0} is already consumed")]
    NoteAlreadyConsumed(NoteId),
    #[error("not with id {0} not found in data store")]
    NoteNotFound(NoteId),
    /// Custom error variant for implementors of the [`DataStore`](crate::executor::DataStore)
    /// trait.
    #[error("{error_msg}")]
    Custom {
        error_msg: Box<str>,
        // thiserror will return this when calling Error::source on DataStoreError.
        source: Option<Box<dyn Error + Send + Sync + 'static>>,
    },
}

impl DataStoreError {
    /// Creates a custom error from an error message.
    pub fn custom(message: impl Into<String>) -> Self {
        let message: String = message.into();
        Self::Custom { error_msg: message.into(), source: None }
    }

    /// Creates a custom error from an error message and a source error.
    pub fn custom_with_source(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        let message: String = message.into();
        Self::Custom {
            error_msg: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

// AUTHENTICATION ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum AuthenticationError {
    #[error("signature rejected: {0}")]
    RejectedSignature(String),
    #[error("unknown public key: {0}")]
    UnknownPublicKey(String),
    /// Custom error variant for implementors of the
    /// [`TransactionAuthenticatior`](crate::auth::TransactionAuthenticator) trait.
    #[error("{error_msg}")]
    Custom {
        error_msg: Box<str>,
        // thiserror will return this when calling Error::source on DataStoreError.
        source: Option<Box<dyn Error + Send + Sync + 'static>>,
    },
}

impl AuthenticationError {
    /// Creates a custom error from an error message.
    pub fn custom(message: impl Into<String>) -> Self {
        let message: String = message.into();
        Self::Custom { error_msg: message.into(), source: None }
    }

    /// Creates a custom error from an error message and a source error.
    pub fn custom_with_source(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        let message: String = message.into();
        Self::Custom {
            error_msg: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

#[cfg(test)]
mod error_assertions {
    use super::*;

    /// Asserts at compile time that the passed error has Send + Sync + 'static bounds.
    fn _assert_error_is_send_sync_static<E: core::error::Error + Send + Sync + 'static>(_: E) {}

    fn _assert_data_store_error_bounds(err: DataStoreError) {
        _assert_error_is_send_sync_static(err);
    }

    fn _assert_authentication_error_bounds(err: AuthenticationError) {
        _assert_error_is_send_sync_static(err);
    }
}
