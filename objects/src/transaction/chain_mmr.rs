use crate::{
    crypto::merkle::{InnerNodeInfo, MmrPeaks, PartialMmr},
    utils::collections::{BTreeMap, Vec},
    BlockHeader, ChainMmrError,
};

// CHAIN MMR
// ================================================================================================

/// A struct that represents the chain Merkle Mountain Range (MMR).
///
/// The MMR allows for efficient authentication of input notes during transaction execution.
/// Authentication is achieved by providing inclusion proofs for the notes consumed in the
/// transaction against the chain MMR root associated with the latest block known at the time
/// of transaction execution.
///
/// [ChainMmr] represents a partial view into the actual MMR and contains authentication paths
/// for a limited set of blocks. The intent is to include only the blocks relevant for execution
/// of a specific transaction (i.e., the blocks corresponding to all input notes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainMmr {
    /// Partial view of the Chain MMR with authentication paths for the blocks listed below.
    mmr: PartialMmr,
    /// A map of block_num |-> block_header for all blocks for which the partial MMR contains
    /// authentication paths.
    blocks: BTreeMap<u32, BlockHeader>,
}

impl ChainMmr {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new [ChainMmr] instantiated from the provided partial MMR and a list of block
    /// headers.
    ///
    /// # Errors
    /// Returns an error if:
    /// - block_num for any of the blocks is greater than the chain length implied by the provided
    ///   partial MMR.
    /// - The same block appears more than once in the provided list of block headers.
    /// - The partial MMR does not track authentication paths for any of the specified blocks.
    pub fn new(mmr: PartialMmr, blocks: Vec<BlockHeader>) -> Result<Self, ChainMmrError> {
        let chain_length = mmr.forest();

        let mut block_map = BTreeMap::new();
        for block in blocks.into_iter() {
            if block.block_num() as usize >= chain_length {
                return Err(ChainMmrError::block_num_too_big(chain_length, block.block_num()));
            }

            if block_map.insert(block.block_num(), block).is_some() {
                return Err(ChainMmrError::duplicate_block(block.block_num()));
            }

            if !mmr.is_tracked(block.block_num() as usize) {
                return Err(ChainMmrError::untracked_block(block.block_num()));
            }
        }

        Ok(Self { mmr, blocks: block_map })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns peaks of this MMR.
    pub fn peaks(&self) -> MmrPeaks {
        self.mmr.peaks()
    }

    /// Returns total number of blocks contain in the chain described by this MMR.
    pub fn chain_length(&self) -> usize {
        self.mmr.forest()
    }

    /// Returns the block header for the specified block, or None if the block is not present in
    /// this partial MMR.
    pub fn get_block(&self, block_num: u32) -> Option<&BlockHeader> {
        self.blocks.get(&block_num)
    }

    // ITERATORS
    // --------------------------------------------------------------------------------------------

    /// Returns an iterator over the inner nodes of authentication paths contained in this chain
    /// MMR.
    pub fn inner_nodes(&self) -> impl Iterator<Item = InnerNodeInfo> + '_ {
        self.mmr.inner_nodes(
            self.blocks.values().map(|block| (block.block_num() as usize, block.hash())),
        )
    }
}
