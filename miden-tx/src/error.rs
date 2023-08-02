use super::{
    AccountError, AccountId, AssemblyError, Digest, ExecutionError, NodeIndex,
    TransactionResultError,
};
use miden_objects::TransactionWitnessError;
use miden_verifier::VerificationError;

#[derive(Debug)]
pub enum TransactionError {
    TransactionCompilerError(TransactionCompilerError),
    TransactionExecutorError(TransactionExecutorError),
    DataStoreError(DataStoreError),
}

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
    BuildCodeBlockTableFailed(AssemblyError),
}

#[derive(Debug)]
pub enum TransactionExecutorError {
    CompileNoteScriptFailed(TransactionCompilerError),
    CompileTransactionError(TransactionCompilerError),
    ExecuteTransactionProgramFailed(ExecutionError),
    FetchAccountCodeFailed(DataStoreError),
    FetchTransactionDataFailed(DataStoreError),
    LoadAccountFailed(TransactionCompilerError),
    TransactionResultError(TransactionResultError),
}

#[derive(Debug)]
pub enum TransactionProverError {
    ProveTransactionProgramFailed(ExecutionError),
    TransactionResultError(TransactionResultError),
    CorruptTransactionWitnessConsumedNoteData(TransactionWitnessError),
}

#[derive(Debug)]
pub enum TransactionVerifierError {
    TransactionVerificationFailed(VerificationError),
    InsufficientProofSecurityLevel(u32, u32),
}

#[derive(Debug)]
pub enum DataStoreError {
    AccountNotFound(AccountId),
    NoteNotFound(u32, NodeIndex),
}
