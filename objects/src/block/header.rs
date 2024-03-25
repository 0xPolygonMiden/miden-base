use alloc::vec::Vec;

use super::{Digest, Felt, Hasher, ZERO};
use crate::utils::serde::{
    ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable,
};

/// The header of a block. It contains metadata about the block, commitments to the current
/// state of the chain and the hash of the proof that attests to the integrity of the chain.
///
/// A block header includes the following fields:
///
/// - prev_hash is the hash of the previous blocks header.
/// - block_num is a unique sequential number of the current block.
/// - chain_root is a commitment to an MMR of the entire chain where each block is a leaf.
/// - account_root is a commitment to account database.
/// - nullifier_root is a commitment to the nullifier database.
/// - note_root is a commitment to all notes created in the current block.
/// - batch_root is a commitment to a set of transaction batches executed as a part of this block.
/// - proof_hash is a hash of a STARK proof attesting to the correct state transition.
/// - version specifies the version of the protocol.
/// - timestamp is the time when the block was created.
/// - sub_hash is a sequential hash of all fields except the note_root.
/// - hash is a 2-to-1 hash of the sub_hash and the note_root.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct BlockHeader {
    prev_hash: Digest,
    block_num: u32,
    chain_root: Digest,
    account_root: Digest,
    nullifier_root: Digest,
    note_root: Digest,
    batch_root: Digest,
    proof_hash: Digest,
    version: Felt,
    timestamp: Felt,
    sub_hash: Digest,
    hash: Digest,
}

impl BlockHeader {
    /// Creates a new block header.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        prev_hash: Digest,
        block_num: u32,
        chain_root: Digest,
        account_root: Digest,
        nullifier_root: Digest,
        note_root: Digest,
        batch_root: Digest,
        proof_hash: Digest,
        version: Felt,
        timestamp: Felt,
    ) -> Self {
        // compute block sub hash
        let sub_hash = Self::compute_sub_hash(
            prev_hash,
            chain_root,
            account_root,
            nullifier_root,
            batch_root,
            proof_hash,
            version,
            timestamp,
            block_num,
        );

        // The sub hash is merged with the note_root - hash(sub_hash, note_root) to produce the final
        // hash. This is done to make the note_root easily accessible without having to unhash the
        // entire header. Having the note_root easily accessible is useful when authenticating notes.
        let hash = Hasher::merge(&[sub_hash, note_root]);

        Self {
            prev_hash,
            block_num,
            chain_root,
            account_root,
            nullifier_root,
            note_root,
            batch_root,
            proof_hash,
            version,
            timestamp,
            sub_hash,
            hash,
        }
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the hash of the block header.
    pub fn hash(&self) -> Digest {
        self.hash
    }

    /// Returns the sub hash of the block header. The sub hash is a sequential hash of all block
    /// header fields except the note root. This is used in the block hash computation which is a
    /// 2-to-1 hash of the sub hash and the note root [hash(sub_hash, note_root)]. This procedure
    /// is used to make the note root easily accessible without having to unhash the entire header.
    pub fn sub_hash(&self) -> Digest {
        self.sub_hash
    }

    /// Returns the hash of the previous block header.
    pub fn prev_hash(&self) -> Digest {
        self.prev_hash
    }

    /// Returns the block number.
    pub fn block_num(&self) -> u32 {
        self.block_num
    }

    /// Returns the chain root.
    pub fn chain_root(&self) -> Digest {
        self.chain_root
    }

    /// Returns the account database root.
    pub fn account_root(&self) -> Digest {
        self.account_root
    }

    /// Returns the nullifier database root.
    pub fn nullifier_root(&self) -> Digest {
        self.nullifier_root
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

    /// Returns the protocol version.
    pub fn version(&self) -> Felt {
        self.version
    }

    /// Returns the timestamp at which the block was created.
    pub fn timestamp(&self) -> Felt {
        self.timestamp
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Computes the sub hash of the block header.
    ///
    /// The sub hash is computed as a sequential hash of the following fields:
    /// prev_hash, chain_root, account_root, nullifier_root, note_root, batch_root, proof_hash,
    /// version, timestamp, block_num (all fields except the note_root).
    #[allow(clippy::too_many_arguments)]
    fn compute_sub_hash(
        prev_hash: Digest,
        chain_root: Digest,
        account_root: Digest,
        nullifier_root: Digest,
        batch_root: Digest,
        proof_hash: Digest,
        version: Felt,
        timestamp: Felt,
        block_num: u32,
    ) -> Digest {
        let mut elements: Vec<Felt> = Vec::with_capacity(32);
        elements.extend_from_slice(prev_hash.as_elements());
        elements.extend_from_slice(chain_root.as_elements());
        elements.extend_from_slice(account_root.as_elements());
        elements.extend_from_slice(nullifier_root.as_elements());
        elements.extend_from_slice(batch_root.as_elements());
        elements.extend_from_slice(proof_hash.as_elements());
        elements.extend([block_num.into(), version, timestamp, ZERO]);
        elements.resize(32, ZERO);
        Hasher::hash_elements(&elements)
    }
}

#[cfg(feature = "testing")]
mod mock {
    use alloc::vec::Vec;

    use winter_rand_utils as rand;

    use crate::{
        accounts::Account, crypto::merkle::SimpleSmt, BlockHeader, Digest, Felt,
        ACCOUNT_TREE_DEPTH, ZERO,
    };

    impl BlockHeader {
        pub fn mock(
            block_num: u32,
            chain_root: Option<Digest>,
            note_root: Option<Digest>,
            accts: &[Account],
        ) -> Self {
            let acct_db = SimpleSmt::<ACCOUNT_TREE_DEPTH>::with_leaves(
                accts
                    .iter()
                    .flat_map(|acct| {
                        if acct.is_new() {
                            None
                        } else {
                            let felt_id: Felt = acct.id().into();
                            Some((felt_id.as_int(), *acct.hash()))
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .expect("failed to create account db");

            let prev_hash: Digest = rand::rand_array().into();
            let chain_root: Digest = chain_root.unwrap_or(rand::rand_array().into());
            let acct_root: Digest = acct_db.root();
            let nullifier_root: Digest = rand::rand_array().into();
            let note_root: Digest = note_root.unwrap_or(rand::rand_array().into());
            let batch_root: Digest = rand::rand_array().into();
            let proof_hash: Digest = rand::rand_array().into();

            BlockHeader::new(
                prev_hash,
                block_num,
                chain_root,
                acct_root,
                nullifier_root,
                note_root,
                batch_root,
                proof_hash,
                ZERO,
                rand::rand_value(),
            )
        }
    }
}

impl Serializable for BlockHeader {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let Self {
            prev_hash,
            block_num,
            chain_root,
            account_root,
            nullifier_root,
            note_root,
            batch_root,
            proof_hash,
            version,
            timestamp,
            sub_hash,
            hash,
        } = self;

        prev_hash.write_into(target);
        block_num.write_into(target);
        chain_root.write_into(target);
        account_root.write_into(target);
        nullifier_root.write_into(target);
        note_root.write_into(target);
        batch_root.write_into(target);
        proof_hash.write_into(target);
        version.write_into(target);
        timestamp.write_into(target);
        sub_hash.write_into(target);
        hash.write_into(target);
    }
}

impl Deserializable for BlockHeader {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        Ok(Self {
            prev_hash: source.read()?,
            block_num: source.read()?,
            chain_root: source.read()?,
            account_root: source.read()?,
            nullifier_root: source.read()?,
            note_root: source.read()?,
            batch_root: source.read()?,
            proof_hash: source.read()?,
            version: source.read()?,
            timestamp: source.read()?,
            sub_hash: source.read()?,
            hash: source.read()?,
        })
    }
}
