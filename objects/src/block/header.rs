use super::{Digest, Felt, Hasher, ZERO};

/// The header of a block. It contains metadata about the block, commitments to the current
/// state of the chain and the hash of the proof that attests to the integrity of the chain.
///
/// A block header includes the following fields:
///
/// - prev_hash is the hash of the previous blocks header.
/// - block_num is a unique sequential number of the current block.
/// - chain_root is a commitment to an MMR of the entire chain where each block is a leaf.
/// - state_root is a combined commitment to account, and nullifier databases.
/// - note_root is a commitment to all notes created in the current block.
/// - batch_root is a commitment to a set of transaction batches executed as a part of this block.
/// - proof_hash is a hash of a STARK proof attesting to the correct state transition.
pub struct BlockHeader {
    prev_hash: Digest,
    block_num: Felt,
    chain_root: Digest,
    state_root: Digest,
    note_root: Digest,
    batch_root: Digest,
    proof_hash: Digest,
    hash: Digest,
}

impl BlockHeader {
    /// Creates a new block header.
    pub fn new(
        prev_hash: Digest,
        block_num: Felt,
        chain_root: Digest,
        state_root: Digest,
        note_root: Digest,
        batch_root: Digest,
        proof_hash: Digest,
    ) -> Self {
        // compute block hash
        let hash = Self::compute_hash(
            prev_hash, chain_root, state_root, batch_root, proof_hash, block_num, note_root,
        );

        Self {
            prev_hash,
            block_num,
            chain_root,
            state_root,
            note_root,
            batch_root,
            proof_hash,
            hash,
        }
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the hash of the block header.
    pub fn hash(&self) -> Digest {
        self.hash
    }

    /// Returns the hash of the previous block header.
    pub fn prev_hash(&self) -> Digest {
        self.prev_hash
    }

    /// Returns the block number.
    pub fn block_num(&self) -> Felt {
        self.block_num
    }

    /// Returns the chain root.
    pub fn chain_root(&self) -> Digest {
        self.chain_root
    }

    /// Returns the state root.
    pub fn state_root(&self) -> Digest {
        self.state_root
    }

    /// Returns the note root.
    pub fn note_root(&self) -> Digest {
        self.note_root
    }

    /// Returns the batch root.
    pub fn batch_root(&self) -> Digest {
        self.batch_root
    }

    /// Returns the proof hash.
    pub fn proof_hash(&self) -> Digest {
        self.proof_hash
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Computes the hash of the block header.
    ///
    /// The hash is computed as a sequential hash of the following fields:
    /// prev_hash, chain_root, state_root, note_root, batch_root, proof_hash, block_num.
    /// The result is then merged with the note_root - merge(note_root, hash) to produce the final
    /// hash. This is done to make the note_root easily accessible without having to unhash the
    /// entire header. Having the note_root easily accessible is useful when authenticating notes.
    fn compute_hash(
        prev_hash: Digest,
        chain_root: Digest,
        state_root: Digest,
        batch_root: Digest,
        proof_hash: Digest,
        block_num: Felt,
        note_root: Digest,
    ) -> Digest {
        let mut elements: Vec<Felt> = Vec::with_capacity(24);
        elements.extend_from_slice(prev_hash.as_elements());
        elements.extend_from_slice(chain_root.as_elements());
        elements.extend_from_slice(state_root.as_elements());
        elements.extend_from_slice(batch_root.as_elements());
        elements.extend_from_slice(proof_hash.as_elements());
        elements.push(block_num);
        elements.resize(24, ZERO);
        let body_hash = Hasher::hash_elements(&elements);

        Hasher::merge(&[body_hash, note_root])
    }
}

impl From<&BlockHeader> for Vec<Felt> {
    fn from(header: &BlockHeader) -> Self {
        let mut elements: Vec<Felt> = Vec::with_capacity(28);
        elements.extend_from_slice(header.prev_hash.as_elements());
        elements.extend_from_slice(header.chain_root.as_elements());
        elements.extend_from_slice(header.state_root.as_elements());
        elements.extend_from_slice(header.batch_root.as_elements());
        elements.extend_from_slice(header.proof_hash.as_elements());
        elements.push(header.block_num);
        elements.resize(24, ZERO);
        elements.extend_from_slice(header.note_root.as_elements());
        elements
    }
}
