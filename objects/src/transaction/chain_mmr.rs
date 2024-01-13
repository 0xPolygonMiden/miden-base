use super::Digest;
use crate::{
    crypto::merkle::{InnerNodeInfo, MmrPeaks, PartialMmr},
    utils::collections::{BTreeMap, Vec},
    ChainMmrError,
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
    /// A list of `(block_num, block_hash)` tuples for all blocks for which the partial MMR
    /// contains authentication paths.
    blocks: Vec<(usize, Digest)>,
}

impl ChainMmr {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new [ChainMmr] instantiated from the provided partial MMR and a map mapping
    /// block_num |-> block_hash.
    ///
    /// # Errors
    /// Returns an error if maximum block_num is greater than the chain length implied by the
    /// provided partial MMR.
    pub fn new(mmr: PartialMmr, blocks: BTreeMap<u32, Digest>) -> Result<Self, ChainMmrError> {
        let chain_length = mmr.forest();
        let max_block_num = blocks.keys().next_back().cloned().unwrap_or_default() as usize;
        if max_block_num >= chain_length {
            return Err(ChainMmrError::block_num_too_big(chain_length, max_block_num));
        }

        let blocks = blocks.into_iter().map(|(key, val)| (key as usize, val)).collect();
        Ok(Self { mmr, blocks })
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

    // ITERATORS
    // --------------------------------------------------------------------------------------------

    /// Returns an iterator over the inner nodes of authentication paths contained in this chain
    /// MMR.
    pub fn inner_nodes(&self) -> impl Iterator<Item = InnerNodeInfo> + '_ {
        self.mmr.inner_nodes(self.blocks.iter().cloned())
    }
}
