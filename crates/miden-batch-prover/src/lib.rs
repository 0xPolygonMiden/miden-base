#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod proven_batch;
pub use proven_batch::ProvenBatch;

mod proposed_batch;
pub use proposed_batch::ProposedBatch;

mod error;
pub use error::BatchError;

mod local_batch_prover;
pub use local_batch_prover::LocalBatchProver;

#[cfg(test)]
pub mod testing;
