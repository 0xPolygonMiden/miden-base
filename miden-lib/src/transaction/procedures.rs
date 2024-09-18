use alloc::vec::Vec;
use std::sync::LazyLock;

use miden_objects::{utils::Deserializable, Digest, Felt, Hasher};

use super::TransactionKernel;

// CONSTANTS
// ================================================================================================

/// Number of currently used kernel versions.
const NUM_VERSIONS: usize = 1;

// Include file with kernel 0 procedure hashes generated in build.rs
const PROCEDURES_V0: &[u8] = include_bytes!("../../kernel_procs_v0.bin");

/// Array of all available kernels.
pub static PROCEDURES: [LazyLock<Vec<Felt>>; NUM_VERSIONS] =
    [LazyLock::new(|| Vec::read_from_bytes(PROCEDURES_V0).unwrap())];

// TRANSACTION KERNEL
// ================================================================================================

impl TransactionKernel {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// Number of currently used kernel versions.
    pub const NUM_VERSIONS: usize = NUM_VERSIONS;

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns procedures of the kernel specified by the `kernel_version` as vector of Felts.
    pub fn procedures_as_elements(kernel_version: u8) -> Vec<Felt> {
        PROCEDURES
            .get(kernel_version as usize)
            .expect("provided kernel index is out of bounds")
            .to_vec()
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
