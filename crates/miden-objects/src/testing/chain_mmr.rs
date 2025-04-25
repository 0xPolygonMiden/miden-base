use alloc::vec::Vec;

use crate::{
    ChainMmrError,
    block::{BlockChain, BlockHeader},
    transaction::ChainMmr,
};

impl ChainMmr {
    /// Converts the [`BlockChain`] into a [`ChainMmr`] by selectively copying all leaves that are
    /// in the given `blocks` iterator.
    ///
    /// This tracks all blocks in the given iterator in the [`ChainMmr`] except for the block whose
    /// block number equals [`BlockChain::chain_tip`], which is the current chain tip.
    ///
    /// # Panics
    ///
    /// Due to being only available in test scenarios, this function panics when one of the given
    /// blocks does not exist in the provided chain or if the chain does not contain at least the
    /// genesis block.
    pub fn from_blockchain(
        chain: &BlockChain,
        blocks: impl IntoIterator<Item = BlockHeader>,
    ) -> Result<ChainMmr, ChainMmrError> {
        let block_headers: Vec<_> = blocks.into_iter().collect();

        // We take the state at the latest block which will be used as the reference block by
        // transaction or batch kernels. That way, the returned partial mmr's hash peaks will match
        // the chain commitment of the reference block.
        let latest_block = chain
            .chain_tip()
            .expect("block chain should contain at least the genesis block");

        let partial_mmr = chain
            .partial_mmr_from_blocks(
                &block_headers.iter().map(BlockHeader::block_num).collect(),
                latest_block,
            )
            .expect("latest block should be in the chain and set of blocks should be valid");

        ChainMmr::new(partial_mmr, block_headers)
    }
}
