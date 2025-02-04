use miden_crypto::merkle::{Mmr, PartialMmr};

use crate::{block::BlockHeader, transaction::ChainMmr, ChainMmrError};

impl ChainMmr {
    /// Converts the [`Mmr`] into a [`ChainMmr`] by selectively copying all leaves that are in the
    /// given `blocks` iterator.
    ///
    /// This tracks all blocks in the given iterator in the [`ChainMmr`] except for the block whose
    /// block number equals [`Mmr::forest`], which is the current chain length.
    ///
    /// # Panics
    ///
    /// Due to being only available in test scenarios, this function panics when one of the given
    /// blocks does not exist in the provided mmr.
    pub fn from_mmr<I>(
        mmr: &Mmr,
        blocks: impl IntoIterator<Item = BlockHeader, IntoIter = I> + Clone,
    ) -> Result<ChainMmr, ChainMmrError>
    where
        I: Iterator<Item = BlockHeader>,
    {
        // We do not include the latest block as it is used as the reference block and is added to
        // the MMR by the transaction or batch kernel.

        let target_forest = mmr.forest() - 1;
        let peaks = mmr
            .peaks_at(target_forest)
            .expect("target_forest should be smaller than forest of the mmr");
        let mut partial_mmr = PartialMmr::from_peaks(peaks);

        for block_num in blocks
            .clone()
            .into_iter()
            .map(|header| header.block_num().as_usize())
            .filter(|block_num| *block_num < target_forest)
        {
            let leaf = mmr.get(block_num).expect("error: block num does not exist");
            let path =
                mmr.open_at(block_num, target_forest).expect("error: block proof").merkle_path;
            partial_mmr.track(block_num, leaf, &path).expect("error: partial mmr track");
        }

        ChainMmr::new(partial_mmr, blocks)
    }
}
