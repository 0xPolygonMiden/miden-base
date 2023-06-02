use assembly::{
    Assembler, AssemblyContext, AssemblyContextType, AssemblyError, ModuleAst, ProgramAst,
};
use crypto::hash::rpo::RpoDigest as Digest;
use hashbrown::HashMap;
use miden_core::{code_blocks::CodeBlock, Kernel, Operation, Program};
use miden_lib::{MidenLib, TransactionKernel};
use miden_objects::{
    notes::{Note, NoteScript},
    transaction::CompiledTransaction,
    AccountCode, AccountError, AccountId,
};
use miden_stdlib::StdLibrary;

mod compiler;
pub use compiler::{NoteTarget, TransactionComplier};
mod error;
use error::TransactionError;

#[cfg(test)]
mod tests;
