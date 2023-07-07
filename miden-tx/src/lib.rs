use assembly::{
    ast::{ModuleAst, ProgramAst},
    Assembler, AssemblyContext, AssemblyContextType, AssemblyError,
};
use crypto::{hash::rpo::RpoDigest as Digest, merkle::NodeIndex};
use miden_core::{code_blocks::CodeBlock, utils::collections::BTreeMap, Operation, Program};
use miden_lib::{MidenLib, SatKernel};
use miden_objects::{
    notes::{Note, NoteOrigin, NoteScript},
    transaction::{PreparedTransaction, ProvenTransaction, TransactionResult, TransactionWitness},
    Account, AccountCode, AccountError, AccountId, BlockHeader, ChainMmr, TransactionResultError,
};
use miden_stdlib::StdLibrary;
use processor::{ExecutionError, MemAdviceProvider, RecAdviceProvider};

mod compiler;
pub use compiler::{NoteTarget, TransactionComplier};
mod data;
use data::DataStore;
mod error;
mod executor;
pub use error::{
    DataStoreError, TransactionCompilerError, TransactionExecutorError, TransactionProverError,
};
pub use executor::TransactionExecutor;
mod prover;
pub use prover::TransactionProver;

#[cfg(test)]
mod tests;
