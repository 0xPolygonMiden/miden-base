use alloc::collections::BTreeMap;

use vm_core::utils::{Deserializable, Serializable};

use crate::{
    block::{BlockHeader, BlockNumber},
    crypto::merkle::{InnerNodeInfo, MmrPeaks, PartialMmr},
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
/// of a specific transaction (i.e., the blocks corresponding to all input notes and the one needed
/// to validate the seed of a new account, if applicable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainMmr {
    /// Partial view of the Chain MMR with authentication paths for the blocks listed below.
    mmr: PartialMmr,
    /// A map of block_num |-> block_header for all blocks for which the partial MMR contains
    /// authentication paths.
    blocks: BTreeMap<BlockNumber, BlockHeader>,
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
    pub fn new(
        mmr: PartialMmr,
        blocks: impl IntoIterator<Item = BlockHeader>,
    ) -> Result<Self, ChainMmrError> {
        let chain_length = mmr.forest();
        let mut block_map = BTreeMap::new();
        for block in blocks {
            let block_num = block.block_num();
            if block.block_num().as_usize() >= chain_length {
                return Err(ChainMmrError::block_num_too_big(chain_length, block_num));
            }

            if !mmr.is_tracked(block_num.as_usize()) {
                return Err(ChainMmrError::untracked_block(block_num));
            }

            if block_map.insert(block_num, block).is_some() {
                return Err(ChainMmrError::duplicate_block(block_num));
            }
        }

        Ok(Self { mmr, blocks: block_map })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the underlying [`PartialMmr`].
    pub fn mmr(&self) -> &PartialMmr {
        &self.mmr
    }

    /// Returns peaks of this MMR.
    pub fn peaks(&self) -> MmrPeaks {
        self.mmr.peaks()
    }

    /// Returns total number of blocks contain in the chain described by this MMR.
    pub fn chain_length(&self) -> BlockNumber {
        BlockNumber::from(
            u32::try_from(self.mmr.forest())
                .expect("chain mmr should never contain more than u32::MAX blocks"),
        )
    }

    /// Returns true if the block is present in this chain MMR.
    pub fn contains_block(&self, block_num: BlockNumber) -> bool {
        self.blocks.contains_key(&block_num)
    }

    /// Returns the block header for the specified block, or None if the block is not present in
    /// this chain MMR.
    pub fn get_block(&self, block_num: BlockNumber) -> Option<&BlockHeader> {
        self.blocks.get(&block_num)
    }

    /// Returns an iterator over the block headers in this chain MMR.
    pub fn block_headers(&self) -> impl Iterator<Item = &BlockHeader> {
        self.blocks.values()
    }

    // DATA MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Appends the provided block header to this chain MMR. This method assumes that the provided
    /// block header is for the next block in the chain.
    ///
    /// If `track` parameter is set to true, the authentication path for the provided block header
    /// will be added to this chain MMR.
    ///
    /// # Panics
    /// Panics if the `block_header.block_num` is not equal to the current chain length (i.e., the
    /// provided block header is not the next block in the chain).
    pub fn add_block(&mut self, block_header: BlockHeader, track: bool) {
        assert_eq!(block_header.block_num(), self.chain_length());
        self.mmr.add(block_header.hash(), track);
    }

    // ITERATORS
    // --------------------------------------------------------------------------------------------

    /// Returns an iterator over the inner nodes of authentication paths contained in this chain
    /// MMR.
    pub fn inner_nodes(&self) -> impl Iterator<Item = InnerNodeInfo> + '_ {
        self.mmr.inner_nodes(
            self.blocks.values().map(|block| (block.block_num().as_usize(), block.hash())),
        )
    }

    // TESTING
    // --------------------------------------------------------------------------------------------

    /// Returns a mutable reference to the map of block numbers to block headers in this chain MMR.
    ///
    /// Allows mutating the inner map for testing purposes.
    #[cfg(any(feature = "testing", test))]
    pub fn block_headers_mut(&mut self) -> &mut BTreeMap<BlockNumber, BlockHeader> {
        &mut self.blocks
    }

    /// Returns a mutable reference to the partial MMR of this chain MMR.
    ///
    /// Allows mutating the inner partial MMR for testing purposes.
    #[cfg(any(feature = "testing", test))]
    pub fn partial_mmr_mut(&mut self) -> &mut PartialMmr {
        &mut self.mmr
    }
}

impl Serializable for ChainMmr {
    fn write_into<W: miden_crypto::utils::ByteWriter>(&self, target: &mut W) {
        self.mmr.write_into(target);
        self.blocks.write_into(target);
    }
}

impl Deserializable for ChainMmr {
    fn read_from<R: miden_crypto::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, miden_crypto::utils::DeserializationError> {
        let mmr = PartialMmr::read_from(source)?;
        let blocks = BTreeMap::<BlockNumber, BlockHeader>::read_from(source)?;
        Ok(Self { mmr, blocks })
    }
}
// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use vm_core::utils::{Deserializable, Serializable};

    use super::ChainMmr;
    use crate::{
        alloc::vec::Vec,
        block::{BlockHeader, BlockNumber},
        crypto::merkle::{Mmr, PartialMmr},
        Digest,
    };

    #[test]
    fn test_chain_mmr_add() {
        // create chain MMR with 3 blocks - i.e., 2 peaks
        let mut mmr = Mmr::default();
        for i in 0..3 {
            let block_header = int_to_block_header(i);
            mmr.add(block_header.hash());
        }
        let partial_mmr: PartialMmr = mmr.peaks().into();
        let mut chain_mmr = ChainMmr::new(partial_mmr, Vec::new()).unwrap();

        // add a new block to the chain MMR, this reduces the number of peaks to 1
        let block_num = 3;
        let bock_header = int_to_block_header(block_num);
        mmr.add(bock_header.hash());
        chain_mmr.add_block(bock_header, true);

        assert_eq!(
            mmr.open(block_num as usize).unwrap(),
            chain_mmr.mmr.open(block_num as usize).unwrap().unwrap()
        );

        // add one more block to the chain MMR, the number of peaks is again 2
        let block_num = 4;
        let bock_header = int_to_block_header(block_num);
        mmr.add(bock_header.hash());
        chain_mmr.add_block(bock_header, true);

        assert_eq!(
            mmr.open(block_num as usize).unwrap(),
            chain_mmr.mmr.open(block_num as usize).unwrap().unwrap()
        );

        // add one more block to the chain MMR, the number of peaks is still 2
        let block_num = 5;
        let bock_header = int_to_block_header(block_num);
        mmr.add(bock_header.hash());
        chain_mmr.add_block(bock_header, true);

        assert_eq!(
            mmr.open(block_num as usize).unwrap(),
            chain_mmr.mmr.open(block_num as usize).unwrap().unwrap()
        );
    }

    #[test]
    fn tst_chain_mmr_serialization() {
        // create chain MMR with 3 blocks - i.e., 2 peaks
        let mut mmr = Mmr::default();
        for i in 0..3 {
            let block_header = int_to_block_header(i);
            mmr.add(block_header.hash());
        }
        let partial_mmr: PartialMmr = mmr.peaks().into();
        let chain_mmr = ChainMmr::new(partial_mmr, Vec::new()).unwrap();

        let bytes = chain_mmr.to_bytes();
        let deserialized = ChainMmr::read_from_bytes(&bytes).unwrap();

        assert_eq!(chain_mmr, deserialized);
    }

    fn int_to_block_header(block_num: impl Into<BlockNumber>) -> BlockHeader {
        BlockHeader::new(
            0,
            Digest::default(),
            block_num.into(),
            Digest::default(),
            Digest::default(),
            Digest::default(),
            Digest::default(),
            Digest::default(),
            Digest::default(),
            Digest::default(),
            0,
        )
    }
}
