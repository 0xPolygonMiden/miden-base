use super::{AccountError, AccountId, AssemblyError, Digest, ExecutionError, NodeIndex};

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
}

#[derive(Debug)]
pub enum DataStoreError {
    AccountNotFound(AccountId),
    NoteNotFound(u32, NodeIndex),
}
