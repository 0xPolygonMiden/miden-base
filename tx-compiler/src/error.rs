use super::{AccountError, AccountId, AssemblyError, Digest};

#[derive(Debug)]
pub enum TransactionError {
    NoNotesProvided,
    LoadAccountFailed(AccountError),
    AccountInterfaceNotFound(AccountId),
    NoteIncompatibleWithAccountInterface(Digest),
    CompileNoteScriptFailed,
    CompileTxScriptFailed(AssemblyError),
    BuildCodeBlockTableFailed(AssemblyError),
}
