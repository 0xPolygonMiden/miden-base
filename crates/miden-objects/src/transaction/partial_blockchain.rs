use alloc::{collections::BTreeMap, vec::Vec};

use crate::{
    PartialBlockchainError,
    block::{BlockHeader, BlockNumber},
    crypto::merkle::{InnerNodeInfo, MmrPeaks, PartialMmr},
    utils::serde::{Deserializable, Serializable},
};

// PARTIAL BLOCKCHAIN
// ================================================================================================

/// A partial view into the full [`Blockchain`](crate::block::Blockchain)'s Merkle Mountain Range
/// (MMR).
///
/// It allows for efficient authentication of input notes during transaction execution or
/// authentication of reference blocks during batch or block execution. Authentication is achieved
/// by providing inclusion proofs for the notes consumed in the transaction against the partial
/// blockchain root associated with the transaction's reference block.
///
/// [`PartialBlockchain`] contains authentication paths for a limited set of blocks. The intent is
/// to include only the blocks relevant for execution:
/// - For transactions: the set of blocks in which all input notes were created.
/// - For batches: the set of reference blocks of all transactions in the batch and the blocks to
///   prove any unauthenticated note's inclusion.
/// - For blocks: the set of reference blocks of all batches in the block and the blocks to prove
///   any unauthenticated note's inclusion.
///
/// # Guarantees
///
/// The [`PartialBlockchain`] contains the full authenticated [`BlockHeader`]s of all blocks it
/// tracks in its partial MMR and users of this type can make this assumption. This is ensured when
/// using [`PartialBlockchain::new`]. [`PartialBlockchain::new_unchecked`] should only be used
/// whenever this guarantee can be upheld.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialBlockchain {
    /// Partial view of the blockchain with authentication paths for the blocks listed below.
    mmr: PartialMmr,
    /// A map of block_num |-> block_header for all blocks for which the partial MMR contains
    /// authentication paths.
    blocks: BTreeMap<BlockNumber, BlockHeader>,
}

impl PartialBlockchain {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new [PartialBlockchain] instantiated from the provided partial MMR and a list of
    /// block headers.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - block_num for any of the blocks is greater than the chain length implied by the provided
    ///   partial MMR.
    /// - The same block appears more than once in the provided list of block headers.
    /// - The partial MMR does not track authentication paths for any of the specified blocks.
    /// - Any of the provided block header's commitment is not tracked in the MMR, i.e. its
    ///   inclusion cannot be verified.
    pub fn new(
        mmr: PartialMmr,
        blocks: impl IntoIterator<Item = BlockHeader>,
    ) -> Result<Self, PartialBlockchainError> {
        let partial_chain = Self::new_unchecked(mmr, blocks)?;

        // Verify inclusion of all provided blocks in the partial MMR.
        for (block_num, block) in partial_chain.blocks.iter() {
            // SAFETY: new_unchecked returns an error if a block is not tracked in the MMR, so
            // retrieving a proof here should succeed.
            let proof = partial_chain
                .mmr
                .open(block_num.as_usize())
                .expect("block should not exceed chain length")
                .expect("block should be tracked in the partial MMR");

            partial_chain.mmr.peaks().verify(block.commitment(), proof).map_err(|source| {
                PartialBlockchainError::BlockHeaderCommitmentMismatch {
                    block_num: *block_num,
                    block_commitment: block.commitment(),
                    source,
                }
            })?;
        }

        Ok(partial_chain)
    }

    /// Returns a new [PartialBlockchain] instantiated from the provided partial MMR and a list of
    /// block headers.
    ///
    /// # Warning
    ///
    /// This does not verify that the provided block commitments are in the MMR. Use [`Self::new`]
    /// to run this verification. This constructor is provided to bypass this check in trusted
    /// environment because it is relatively expensive.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - block_num for any of the blocks is greater than the chain length implied by the provided
    ///   partial MMR.
    /// - The same block appears more than once in the provided list of block headers.
    /// - The partial MMR does not track authentication paths for any of the specified blocks.
    pub fn new_unchecked(
        mmr: PartialMmr,
        blocks: impl IntoIterator<Item = BlockHeader>,
    ) -> Result<Self, PartialBlockchainError> {
        let chain_length = mmr.forest();
        let mut block_map = BTreeMap::new();
        for block in blocks {
            let block_num = block.block_num();
            if block.block_num().as_usize() >= chain_length {
                return Err(PartialBlockchainError::block_num_too_big(chain_length, block_num));
            }

            // Note that this only checks if a leaf exists at that position but it doesn't
            // assert that it matches the block's commitment provided in the iterator.
            if !mmr.is_tracked(block_num.as_usize()) {
                return Err(PartialBlockchainError::untracked_block(block_num));
            }

            if block_map.insert(block_num, block).is_some() {
                return Err(PartialBlockchainError::duplicate_block(block_num));
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
                .expect("partial blockchain should never contain more than u32::MAX blocks"),
        )
    }

    /// Returns `true` if a block with the given number is present in this partial blockchain.
    ///
    /// Note that this only checks whether an entry with the block's number exists in the MMR.
    pub fn contains_block(&self, block_num: BlockNumber) -> bool {
        self.blocks.contains_key(&block_num)
    }

    /// Returns the block header for the specified block, or None if the block is not present in
    /// this partial blockchain.
    pub fn get_block(&self, block_num: BlockNumber) -> Option<&BlockHeader> {
        self.blocks.get(&block_num)
    }

    /// Returns an iterator over the block headers in this partial blockchain.
    pub fn block_headers(&self) -> impl Iterator<Item = &BlockHeader> {
        self.blocks.values()
    }

    // DATA MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Appends the provided block header to this partial blockchain. This method assumes that the
    /// provided block header is for the next block in the chain.
    ///
    /// If `track` parameter is set to true, the authentication path for the provided block header
    /// will be added to this partial blockchain.
    ///
    /// # Panics
    /// Panics if the `block_header.block_num` is not equal to the current chain length (i.e., the
    /// provided block header is not the next block in the chain).
    pub fn add_block(&mut self, block_header: BlockHeader, track: bool) {
        assert_eq!(block_header.block_num(), self.chain_length());
        self.mmr.add(block_header.commitment(), track);
    }

    // ITERATORS
    // --------------------------------------------------------------------------------------------

    /// Returns an iterator over the inner nodes of authentication paths contained in this chain
    /// MMR.
    pub fn inner_nodes(&self) -> impl Iterator<Item = InnerNodeInfo> + '_ {
        self.mmr.inner_nodes(
            self.blocks
                .values()
                .map(|block| (block.block_num().as_usize(), block.commitment())),
        )
    }

    // TESTING
    // --------------------------------------------------------------------------------------------

    /// Returns a mutable reference to the map of block numbers to block headers in this partial
    /// blockchain.
    ///
    /// Allows mutating the inner map for testing purposes.
    #[cfg(any(feature = "testing", test))]
    pub fn block_headers_mut(&mut self) -> &mut BTreeMap<BlockNumber, BlockHeader> {
        &mut self.blocks
    }

    /// Returns a mutable reference to the partial MMR of this partial blockchain.
    ///
    /// Allows mutating the inner partial MMR for testing purposes.
    #[cfg(any(feature = "testing", test))]
    pub fn partial_mmr_mut(&mut self) -> &mut PartialMmr {
        &mut self.mmr
    }
}

impl Serializable for PartialBlockchain {
    fn write_into<W: miden_crypto::utils::ByteWriter>(&self, target: &mut W) {
        self.mmr.write_into(target);
        self.blocks.write_into(target);
    }
}

impl Deserializable for PartialBlockchain {
    fn read_from<R: miden_crypto::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, miden_crypto::utils::DeserializationError> {
        let mmr = PartialMmr::read_from(source)?;
        let blocks = BTreeMap::<BlockNumber, BlockHeader>::read_from(source)?;
        Ok(Self { mmr, blocks })
    }
}

impl Default for PartialBlockchain {
    fn default() -> Self {
        // TODO: Replace with PartialMmr::default after https://github.com/0xMiden/crypto/pull/409/files
        // is available for use.
        let empty_mmr = PartialMmr::from_peaks(
            MmrPeaks::new(0, Vec::new()).expect("empty MmrPeaks should be valid"),
        );

        Self::new(empty_mmr, Vec::new()).expect("empty partial blockchain should be valid")
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use vm_core::utils::{Deserializable, Serializable};

    use super::PartialBlockchain;
    use crate::{
        Digest, PartialBlockchainError,
        alloc::vec::Vec,
        block::{BlockHeader, BlockNumber},
        crypto::merkle::{Mmr, PartialMmr},
    };

    #[test]
    fn test_partial_blockchain_add() {
        // create partial blockchain with 3 blocks - i.e., 2 peaks
        let mut mmr = Mmr::default();
        for i in 0..3 {
            let block_header = int_to_block_header(i);
            mmr.add(block_header.commitment());
        }
        let partial_mmr: PartialMmr = mmr.peaks().into();
        let mut partial_blockchain = PartialBlockchain::new(partial_mmr, Vec::new()).unwrap();

        // add a new block to the partial blockchain, this reduces the number of peaks to 1
        let block_num = 3;
        let bock_header = int_to_block_header(block_num);
        mmr.add(bock_header.commitment());
        partial_blockchain.add_block(bock_header, true);

        assert_eq!(
            mmr.open(block_num as usize).unwrap(),
            partial_blockchain.mmr.open(block_num as usize).unwrap().unwrap()
        );

        // add one more block to the partial blockchain, the number of peaks is again 2
        let block_num = 4;
        let bock_header = int_to_block_header(block_num);
        mmr.add(bock_header.commitment());
        partial_blockchain.add_block(bock_header, true);

        assert_eq!(
            mmr.open(block_num as usize).unwrap(),
            partial_blockchain.mmr.open(block_num as usize).unwrap().unwrap()
        );

        // add one more block to the partial blockchain, the number of peaks is still 2
        let block_num = 5;
        let bock_header = int_to_block_header(block_num);
        mmr.add(bock_header.commitment());
        partial_blockchain.add_block(bock_header, true);

        assert_eq!(
            mmr.open(block_num as usize).unwrap(),
            partial_blockchain.mmr.open(block_num as usize).unwrap().unwrap()
        );
    }

    #[test]
    fn partial_blockchain_new_on_invalid_header_fails() {
        let block_header0 = int_to_block_header(0);
        let block_header1 = int_to_block_header(1);
        let block_header2 = int_to_block_header(2);

        let mut mmr = Mmr::default();
        mmr.add(block_header0.commitment());
        mmr.add(block_header1.commitment());
        mmr.add(block_header2.commitment());

        let mut partial_mmr = PartialMmr::from_peaks(mmr.peaks());
        for i in 0..3 {
            partial_mmr
                .track(i, mmr.get(i).unwrap(), &mmr.open(i).unwrap().merkle_path)
                .unwrap();
        }

        let fake_block_header2 = BlockHeader::mock(2, None, None, &[], Digest::default());

        assert_ne!(block_header2.commitment(), fake_block_header2.commitment());

        // Construct a PartialBlockchain with an invalid block header.
        let error = PartialBlockchain::new(
            partial_mmr,
            vec![block_header0, block_header1, fake_block_header2.clone()],
        )
        .unwrap_err();

        assert_matches!(
            error,
            PartialBlockchainError::BlockHeaderCommitmentMismatch {
                block_commitment,
                block_num,
                ..
            } if block_commitment == fake_block_header2.commitment() && block_num == fake_block_header2.block_num()
        )
    }

    #[test]
    fn partial_blockchain_new_on_block_number_exceeding_chain_length_fails() {
        let block_header0 = int_to_block_header(0);
        let mmr = Mmr::default();
        let partial_mmr = PartialMmr::from_peaks(mmr.peaks());

        let error = PartialBlockchain::new(partial_mmr, [block_header0]).unwrap_err();

        assert_matches!(error, PartialBlockchainError::BlockNumTooBig {
          chain_length,
          block_num,
        } if chain_length == 0 && block_num == BlockNumber::from(0));
    }

    #[test]
    fn partial_blockchain_new_on_untracked_block_number_fails() {
        let block_header0 = int_to_block_header(0);
        let block_header1 = int_to_block_header(1);

        let mut mmr = Mmr::default();
        mmr.add(block_header0.commitment());
        mmr.add(block_header1.commitment());

        let mut partial_mmr = PartialMmr::from_peaks(mmr.peaks());
        partial_mmr
            .track(1, block_header1.commitment(), &mmr.open(1).unwrap().merkle_path)
            .unwrap();

        let error =
            PartialBlockchain::new(partial_mmr, [block_header0, block_header1]).unwrap_err();

        assert_matches!(error, PartialBlockchainError::UntrackedBlock {
          block_num,
        } if block_num == BlockNumber::from(0));
    }

    #[test]
    fn partial_blockchain_serialization() {
        // create partial blockchain with 3 blocks - i.e., 2 peaks
        let mut mmr = Mmr::default();
        for i in 0..3 {
            let block_header = int_to_block_header(i);
            mmr.add(block_header.commitment());
        }
        let partial_mmr: PartialMmr = mmr.peaks().into();
        let partial_blockchain = PartialBlockchain::new(partial_mmr, Vec::new()).unwrap();

        let bytes = partial_blockchain.to_bytes();
        let deserialized = PartialBlockchain::read_from_bytes(&bytes).unwrap();

        assert_eq!(partial_blockchain, deserialized);
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
