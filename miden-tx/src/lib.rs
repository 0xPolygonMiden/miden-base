#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

use miden_lib::transaction::TransactionKernel;
pub use miden_objects::transaction::TransactionInputs;
use miden_objects::{
    accounts::{AccountCode, AccountId},
    notes::{NoteId, NoteScript},
    transaction::{ExecutedTransaction, PreparedTransaction},
    vm::{CodeBlock, Program},
    AccountError, Digest,
};
use vm_processor::{ExecutionError, RecAdviceProvider};

mod compiler;
pub use compiler::{ScriptTarget, TransactionCompiler};

mod executor;
pub use executor::{DataStore, TransactionExecutor};

pub mod host;
pub use host::{TransactionHost, TransactionProgress};

mod prover;
pub use prover::{ProvingOptions, TransactionProver};

mod verifier;
pub use verifier::TransactionVerifier;

mod error;
pub use error::{
    DataStoreError, TransactionCompilerError, TransactionExecutorError, TransactionProverError,
    TransactionVerifierError,
};

#[cfg(test)]
mod tests;

// RE-EXPORTS
// ================================================================================================
pub use miden_objects::utils;
