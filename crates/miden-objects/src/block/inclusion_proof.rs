use miden_crypto::merkle::MerklePath;

use crate::block::{BlockHeader, BlockNumber};

// TODO: Document.
/// Data required to verify a block's inclusion proof.
#[derive(Clone, Debug)]
pub struct BlockInclusionProof {
    pub block_header: BlockHeader,
    pub mmr_path: MerklePath,
    pub chain_length: BlockNumber,
}
