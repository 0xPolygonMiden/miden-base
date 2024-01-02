use core::fmt;

use miden_objects::{assembly::AssemblyError, crypto::merkle::NodeIndex, TransactionOutputError};
use miden_verifier::VerificationError;

use super::{AccountError, AccountId, Digest, ExecutionError};

// TRANSACTION COMPILER ERROR
// ================================================================================================
#[derive(Debug)]
pub enum TransactionCompilerError {
    InvalidTransactionInputs,
    LoadAccountFailed(AccountError),
    AccountInterfaceNotFound(AccountId),
    ProgramIncompatibleWithAccountInterface(Digest),
    NoteIncompatibleWithAccountInterface(Digest),
    TxScriptIncompatibleWithAccountInterface(Digest),
    CompileNoteScriptFailed,
    CompileTxScriptFailed(AssemblyError),
    CompileTxScriptFailedUnknown,
    BuildCodeBlockTableFailed(AssemblyError),
}

impl fmt::Display for TransactionCompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionCompilerError {}

// TRANSACTION EXECUTOR ERROR
// ================================================================================================
#[derive(Debug)]
pub enum TransactionExecutorError {
    CompileNoteScriptFailed(TransactionCompilerError),
    CompileTransactionScriptFailed(TransactionCompilerError),
    CompileTransactionError(TransactionCompilerError),
    ConstructPreparedTransactionFailed(miden_objects::TransactionError),
    ExecuteTransactionProgramFailed(ExecutionError),
    ExecutedTransactionConstructionFailed(miden_objects::TransactionError),
    FetchAccountCodeFailed(DataStoreError),
    FetchTransactionInputsFailed(DataStoreError),
    LoadAccountFailed(TransactionCompilerError),
    TransactionOutputError(TransactionOutputError),
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
#[derive(Debug)]
pub enum TransactionProverError {
    ProveTransactionProgramFailed(ExecutionError),
    TransactionOutputError(TransactionOutputError),
}

impl fmt::Display for TransactionProverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionProverError {}

// TRANSACTION VERIFIER ERROR
// ================================================================================================
#[derive(Debug)]
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

// DATA STORE ERROR
// ================================================================================================
#[derive(Debug)]
pub enum DataStoreError {
    AccountNotFound(AccountId),
    NoteNotFound(u32, NodeIndex),
}

impl fmt::Display for DataStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DataStoreError {}
