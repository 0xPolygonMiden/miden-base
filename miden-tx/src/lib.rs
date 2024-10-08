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
    tx_kernel_errors::TX_KERNEL_ERRORS, AuthenticationError, DataStoreError,
    TransactionExecutorError, TransactionProverError, TransactionVerifierError,
};

pub mod auth;

#[cfg(feature = "testing")]
pub mod testing;

#[cfg(test)]
mod tests;

// RE-EXPORTS
// ================================================================================================
pub use miden_objects::utils;
