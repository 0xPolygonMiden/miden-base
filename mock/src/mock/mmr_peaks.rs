use miden_test_utils::crypto::MmrPeaks;

pub fn empty_mmr_peaks() -> MmrPeaks {
    MmrPeaks::new(0, Vec::new()).unwrap()
}
