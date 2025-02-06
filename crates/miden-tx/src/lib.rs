#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub use miden_objects::transaction::TransactionInputs;

mod executor;
pub use executor::{DataStore, TransactionExecutor, TransactionMastStore};

pub mod host;
pub use host::{TransactionHost, TransactionProgress};

mod prover;
pub use prover::{LocalTransactionProver, ProvingOptions, TransactionProver};

mod verifier;
pub use verifier::TransactionVerifier;

mod errors;
pub use errors::{
    DataStoreError, TransactionExecutorError, TransactionProverError, TransactionVerifierError,
};
pub use miden_lib::AuthenticationError;

pub mod auth;

#[cfg(any(feature = "testing", test))]
pub mod testing;

#[cfg(test)]
mod tests;

// RE-EXPORTS
// ================================================================================================
pub use miden_objects::utils;
