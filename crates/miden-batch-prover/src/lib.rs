#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod local_batch_prover;
pub use local_batch_prover::LocalBatchProver;

#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod tests;
