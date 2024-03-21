use alloc::vec::Vec;

use miden_objects::crypto::merkle::MmrPeaks;

pub fn empty_mmr_peaks() -> MmrPeaks {
    MmrPeaks::new(0, Vec::new()).unwrap()
}
