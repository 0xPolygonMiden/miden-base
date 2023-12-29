use miden_lib::transaction::TransactionKernel;
pub use miden_objects::transaction::TransactionInputs;
use miden_objects::{
    accounts::{AccountCode, AccountId},
    notes::{NoteOrigin, NoteScript},
    transaction::{ExecutedTransaction, PreparedTransaction},
    utils::collections::BTreeMap,
    vm::CodeBlock,
    AccountError, Digest,
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
