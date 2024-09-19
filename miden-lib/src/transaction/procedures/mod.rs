use alloc::vec::Vec;

use kernel_v0::KERNEL0_PROCEDURES;
use miden_objects::{Digest, Felt, Hasher};

use super::TransactionKernel;

// Include procedure hashes generated in build.rs
#[rustfmt::skip]
mod kernel_v0;

// TRANSACTION KERNEL
// ================================================================================================

impl TransactionKernel {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// Number of currently used kernel versions.
    pub const NUM_VERSIONS: usize = 1;

    /// Array of all available kernels.
    pub const PROCEDURES: [&'static [Digest]; Self::NUM_VERSIONS] = [&KERNEL0_PROCEDURES];

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns procedures of the kernel specified by the `kernel_version` as vector of Felts.
    pub fn procedures_as_elements(kernel_version: u8) -> Vec<Felt> {
        Digest::digests_as_elements(
            Self::PROCEDURES
                .get(kernel_version as usize)
                .expect("provided kernel index is out of bounds")
                .iter(),
        )
        .cloned()
        .collect::<Vec<Felt>>()
    }

    /// Computes the accumulative hash of all procedures of the kernel specified by the
    /// `kernel_version`.
    pub fn kernel_hash(kernel_version: u8) -> Digest {
        Hasher::hash_elements(&Self::procedures_as_elements(kernel_version))
    }

    /// Computes a hash from all kernel hashes.
    pub fn kernel_root() -> Digest {
        Hasher::hash_elements(&[Self::kernel_hash(0).as_elements()].concat())
    }
}
