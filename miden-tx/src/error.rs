use super::{AccountError, AccountId, AssemblyError, Digest};

#[derive(Debug)]
pub enum TransactionError {
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
