use alloc::string::String;
use core::fmt::{self, Display};

use miden_objects::{
    accounts::AccountId, notes::NoteId, AccountError, Felt, ProvenTransactionError,
    TransactionInputError, TransactionOutputError,
};
use miden_verifier::VerificationError;
use vm_processor::ExecutionError;

pub mod tx_kernel_errors;

// TRANSACTION EXECUTOR ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionExecutorError {
    ExecuteTransactionProgramFailed(ExecutionError),
    FetchTransactionInputsFailed(DataStoreError),
    InconsistentAccountId {
        input_id: AccountId,
        output_id: AccountId,
    },
    InconsistentAccountNonceDelta {
        expected: Option<Felt>,
        actual: Option<Felt>,
    },
    InvalidTransactionOutput(TransactionOutputError),
    TransactionHostCreationFailed(TransactionHostError),
}

impl fmt::Display for TransactionExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionExecutorError {}

// TRANSACTION PROVER ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionProverError {
    ProveTransactionProgramFailed(ExecutionError),
    InvalidAccountDelta(AccountError),
    InvalidTransactionOutput(TransactionOutputError),
    ProvenTransactionError(ProvenTransactionError),
    TransactionHostCreationFailed(TransactionHostError),
}

impl Display for TransactionProverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionProverError::ProveTransactionProgramFailed(inner) => {
                write!(f, "Proving transaction failed: {}", inner)
            },
            TransactionProverError::InvalidAccountDelta(account_error) => {
                write!(f, "Applying account delta failed: {}", account_error)
            },
            TransactionProverError::InvalidTransactionOutput(inner) => {
                write!(f, "Transaction output invalid: {}", inner)
            },
            TransactionProverError::ProvenTransactionError(inner) => {
                write!(f, "Building proven transaction error: {}", inner)
            },
            TransactionProverError::TransactionHostCreationFailed(inner) => {
                write!(f, "Failed to create the transaction host: {}", inner)
            },
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionProverError {}

// TRANSACTION VERIFIER ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionVerifierError {
    TransactionVerificationFailed(VerificationError),
    InsufficientProofSecurityLevel(u32, u32),
}

impl fmt::Display for TransactionVerifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionVerifierError {}

// TRANSACTION HOST ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionHostError {
    AccountProcedureIndexMapError(String),
}

impl fmt::Display for TransactionHostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionHostError {}

// DATA STORE ERROR
// ================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataStoreError {
    AccountNotFound(AccountId),
    BlockNotFound(u32),
    InvalidTransactionInput(TransactionInputError),
    InternalError(String),
    NoteAlreadyConsumed(NoteId),
    NoteNotFound(NoteId),
}

impl fmt::Display for DataStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DataStoreError {}

// AUTHENTICATION ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AuthenticationError {
    InternalError(String),
    RejectedSignature(String),
    UnknownKey(String),
}

impl fmt::Display for AuthenticationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthenticationError::InternalError(error) => {
                write!(f, "authentication internal error: {error}")
            },
            AuthenticationError::RejectedSignature(reason) => {
                write!(f, "signature was rejected: {reason}")
            },
            AuthenticationError::UnknownKey(error) => write!(f, "unknown key error: {error}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AuthenticationError {}
