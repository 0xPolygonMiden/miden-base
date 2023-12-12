use miden_lib::SatKernel;
use miden_objects::{
    accounts::{Account, AccountCode, AccountId},
    assembly::CodeBlock,
    notes::{NoteOrigin, NoteScript},
    transaction::{PreparedTransaction, TransactionResult},
    utils::collections::BTreeMap,
    AccountError, BlockHeader, ChainMmr, Digest, Hasher, TransactionResultError,
};
use vm_core::Program;
use vm_processor::{ExecutionError, RecAdviceProvider};

mod compiler;
pub use compiler::{ScriptTarget, TransactionCompiler};

mod data;
pub use data::DataStore;

mod executor;
pub use executor::TransactionExecutor;

pub mod host;
pub use host::TransactionHost;

mod prover;
pub use prover::{ProvingOptions, TransactionProver};

mod result;
pub use result::TryFromVmResult;

mod verifier;
pub use verifier::TransactionVerifier;

mod error;
pub use error::{
    DataStoreError, TransactionCompilerError, TransactionError, TransactionExecutorError,
    TransactionProverError, TransactionVerifierError,
};

#[cfg(test)]
mod tests;
