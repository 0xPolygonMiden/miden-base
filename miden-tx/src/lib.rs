use assembly::{
    ast::{ModuleAst, ProgramAst},
    Assembler, AssemblyContext, AssemblyContextType, AssemblyError,
};
use crypto::hash::rpo::RpoDigest as Digest;
use miden_core::{code_blocks::CodeBlock, utils::collections::BTreeMap, Operation, Program};
use miden_lib::{MidenLib, SatKernel};
use miden_objects::{
    notes::{Note, NoteScript},
    transaction::CompiledTransaction,
    AccountCode, AccountError, AccountId,
};
use miden_stdlib::StdLibrary;

mod compiler;
pub use compiler::{NoteTarget, TransactionComplier};
mod error;
pub use error::TransactionError;
