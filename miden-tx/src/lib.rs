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
    TransactionWitnessError,
};
use miden_stdlib::StdLibrary;
use processor::{ExecutionError, MemAdviceProvider, RecAdviceProvider};

mod compiler;
mod data;
mod error;
mod executor;
mod prover;

use data::DataStore;
use error::DataStoreError;

pub use compiler::{NoteTarget, TransactionComplier};
pub use error::{TransactionCompilerError, TransactionExecutorError, TransactionProverError};
pub use executor::TransactionExecutor;
pub use prover::TransactionProver;

#[cfg(any(test, feature = "testing"))]
pub mod mock;
#[cfg(test)]
mod tests;
