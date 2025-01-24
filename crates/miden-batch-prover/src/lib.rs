#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod proposed_batch;
pub use proposed_batch::ProposedBatch;

mod local_batch_prover;
pub use local_batch_prover::LocalBatchProver;

#[cfg(any(feature = "testing", test))]
pub mod testing;
