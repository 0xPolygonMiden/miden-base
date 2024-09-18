use alloc::vec::Vec;

use miden_objects::{utils::Deserializable, Digest, Felt, Hasher};
use once_cell::sync::OnceCell;

use super::TransactionKernel;

// CONSTANTS
// ================================================================================================

/// Number of currently used kernel versions.
const NUM_VERSIONS: usize = 1;

/// Include file with kernel 0 procedure hashes generated in build.rs
const PROCEDURES_RAW: [&[u8]; NUM_VERSIONS] = [include_bytes!("../../kernel_procs_v0.bin")];

/// Array of all available kernels.
pub static PROCEDURES: [OnceCell<Vec<Felt>>; NUM_VERSIONS] = [OnceCell::new()];

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
            .get_or_init(|| {
                Vec::read_from_bytes(PROCEDURES_RAW[kernel_version as usize])
                    .expect("failed to deserialize kernel procedures")
            })
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
