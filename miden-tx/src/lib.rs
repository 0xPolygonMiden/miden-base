use assembly::{
    ast::{ModuleAst, ProgramAst},
    Assembler, AssemblyContext, AssemblyError,
};
use crypto::{hash::rpo::Rpo256 as Hasher, hash::rpo::RpoDigest as Digest, merkle::NodeIndex};
use miden_core::{code_blocks::CodeBlock, utils::collections::BTreeMap, Operation, Program};
use miden_lib::{MidenLib, SatKernel};
use miden_objects::{
    notes::{Note, NoteOrigin, NoteScript},
    transaction::{PreparedTransaction, TransactionResult},
    Account, AccountCode, AccountError, AccountId, BlockHeader, ChainMmr, TransactionResultError,
};
use miden_stdlib::StdLibrary;
use processor::{ExecutionError, RecAdviceProvider};

mod compiler;
pub use compiler::{NoteTarget, TransactionComplier};
mod data;
use data::DataStore;
mod error;
mod executor;
pub use error::TransactionError;
use error::{
    DataStoreError, TransactionCompilerError, TransactionExecutorError, TransactionProverError,
    TransactionVerifierError,
};
pub use executor::TransactionExecutor;
mod prover;
pub use prover::TransactionProver;
mod verifier;
pub use verifier::TransactionVerifier;

#[cfg(test)]
mod tests;
