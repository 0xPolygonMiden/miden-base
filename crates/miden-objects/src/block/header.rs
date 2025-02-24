use alloc::vec::Vec;

use crate::{
    account::AccountId,
    block::BlockNumber,
    transaction::TransactionId,
    utils::serde::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest, Felt, Hasher, ZERO,
};

/// The header of a block. It contains metadata about the block, commitments to the current
/// state of the chain and the hash of the proof that attests to the integrity of the chain.
///
/// A block header includes the following fields:
///
/// - `version` specifies the version of the protocol.
/// - `prev_hash` is the hash of the previous block header.
/// - `block_num` is a unique sequential number of the current block.
/// - `chain_root` is a commitment to an MMR of the entire chain where each block is a leaf.
/// - `account_root` is a commitment to account database.
/// - `nullifier_root` is a commitment to the nullifier database.
/// - `note_root` is a commitment to all notes created in the current block.
/// - `tx_hash` is a commitment to a set of IDs of transactions which affected accounts in the
///   block.
/// - `kernel_root` a commitment to all transaction kernels supported by this block.
/// - `proof_hash` is a hash of a STARK proof attesting to the correct state transition.
/// - `timestamp` is the time when the block was created, in seconds since UNIX epoch. Current
///   representation is sufficient to represent time up to year 2106.
/// - `sub_hash` is a sequential hash of all fields except the note_root.
/// - `hash` is a 2-to-1 hash of the sub_hash and the note_root.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct BlockHeader {
    version: u32,
    prev_hash: Digest,
    block_num: BlockNumber,
    chain_root: Digest,
    account_root: Digest,
    nullifier_root: Digest,
    note_root: Digest,
    tx_hash: Digest,
    kernel_root: Digest,
    proof_hash: Digest,
    timestamp: u32,
    sub_hash: Digest,
    hash: Digest,
}

impl BlockHeader {
    /// Creates a new block header.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        version: u32,
        prev_hash: Digest,
        block_num: BlockNumber,
        chain_root: Digest,
        account_root: Digest,
        nullifier_root: Digest,
        note_root: Digest,
        tx_hash: Digest,
        kernel_root: Digest,
        proof_hash: Digest,
        timestamp: u32,
    ) -> Self {
        // compute block sub hash
        let sub_hash = Self::compute_sub_hash(
            version,
            prev_hash,
            chain_root,
            account_root,
            nullifier_root,
            tx_hash,
            kernel_root,
            proof_hash,
            timestamp,
            block_num,
        );

        // The sub hash is merged with the note_root - hash(sub_hash, note_root) to produce the
        // final hash. This is done to make the note_root easily accessible without having
        // to unhash the entire header. Having the note_root easily accessible is useful
        // when authenticating notes.
        let hash = Hasher::merge(&[sub_hash, note_root]);

        Self {
            version,
            prev_hash,
            block_num,
            chain_root,
            account_root,
            nullifier_root,
            note_root,
            tx_hash,
            kernel_root,
            proof_hash,
            timestamp,
            sub_hash,
            hash,
        }
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the protocol version.
    pub fn version(&self) -> u32 {
        self.version
    }

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
    pub fn block_num(&self) -> BlockNumber {
        self.block_num
    }

    /// Returns the epoch to which this block belongs.
    ///
    /// This is the block number shifted right by [`BlockNumber::EPOCH_LENGTH_EXPONENT`].
    pub fn block_epoch(&self) -> u16 {
        self.block_num.block_epoch()
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

    /// Returns the commitment to all transactions in this block.
    ///
    /// The commitment is computed as sequential hash of (`transaction_id`, `account_id`) tuples.
    /// This makes it possible for the verifier to link transaction IDs to the accounts which
    /// they were executed against.
    pub fn tx_hash(&self) -> Digest {
        self.tx_hash
    }

    /// Returns the kernel root.
    ///
    /// Kernel root is computed as a sequential hash of all kernel hashes.
    pub fn kernel_root(&self) -> Digest {
        self.kernel_root
    }

    /// Returns the proof hash.
    pub fn proof_hash(&self) -> Digest {
        self.proof_hash
    }

    /// Returns the timestamp at which the block was created, in seconds since UNIX epoch.
    pub fn timestamp(&self) -> u32 {
        self.timestamp
    }

    /// Returns the block number of the epoch block to which this block belongs.
    pub fn epoch_block_num(&self) -> BlockNumber {
        BlockNumber::from_epoch(self.block_epoch())
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Computes the sub hash of the block header.
    ///
    /// The sub hash is computed as a sequential hash of the following fields:
    /// `prev_hash`, `chain_root`, `account_root`, `nullifier_root`, `note_root`, `tx_hash`,
    /// `kernel_root`, `proof_hash`, `version`, `timestamp`, `block_num` (all fields except the
    /// `note_root`).
    #[allow(clippy::too_many_arguments)]
    fn compute_sub_hash(
        version: u32,
        prev_hash: Digest,
        chain_root: Digest,
        account_root: Digest,
        nullifier_root: Digest,
        tx_hash: Digest,
        kernel_root: Digest,
        proof_hash: Digest,
        timestamp: u32,
        block_num: BlockNumber,
    ) -> Digest {
        let mut elements: Vec<Felt> = Vec::with_capacity(32);
        elements.extend_from_slice(prev_hash.as_elements());
        elements.extend_from_slice(chain_root.as_elements());
        elements.extend_from_slice(account_root.as_elements());
        elements.extend_from_slice(nullifier_root.as_elements());
        elements.extend_from_slice(tx_hash.as_elements());
        elements.extend_from_slice(kernel_root.as_elements());
        elements.extend_from_slice(proof_hash.as_elements());
        elements.extend([block_num.into(), version.into(), timestamp.into(), ZERO]);
        Hasher::hash_elements(&elements)
    }

    /// Computes a commitment to the provided list of transactions.
    pub fn compute_tx_commitment(
        updated_accounts: impl Iterator<Item = (TransactionId, AccountId)>,
    ) -> Digest {
        let mut elements = vec![];
        for (transaction_id, account_id) in updated_accounts {
            let [account_id_prefix, account_id_suffix] = <[Felt; 2]>::from(account_id);
            elements.extend_from_slice(transaction_id.as_elements());
            elements.extend_from_slice(&[account_id_prefix, account_id_suffix, ZERO, ZERO]);
        }

        Hasher::hash_elements(&elements)
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for BlockHeader {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.version.write_into(target);
        self.prev_hash.write_into(target);
        self.block_num.write_into(target);
        self.chain_root.write_into(target);
        self.account_root.write_into(target);
        self.nullifier_root.write_into(target);
        self.note_root.write_into(target);
        self.tx_hash.write_into(target);
        self.kernel_root.write_into(target);
        self.proof_hash.write_into(target);
        self.timestamp.write_into(target);
    }
}

impl Deserializable for BlockHeader {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let version = source.read()?;
        let prev_hash = source.read()?;
        let block_num = source.read()?;
        let chain_root = source.read()?;
        let account_root = source.read()?;
        let nullifier_root = source.read()?;
        let note_root = source.read()?;
        let tx_hash = source.read()?;
        let kernel_root = source.read()?;
        let proof_hash = source.read()?;
        let timestamp = source.read()?;

        Ok(Self::new(
            version,
            prev_hash,
            block_num,
            chain_root,
            account_root,
            nullifier_root,
            note_root,
            tx_hash,
            kernel_root,
            proof_hash,
            timestamp,
        ))
    }
}

#[cfg(test)]
mod tests {
    use vm_core::Word;
    use winter_rand_utils::rand_array;

    use super::*;

    #[test]
    fn test_serde() {
        let chain_root: Word = rand_array();
        let note_root: Word = rand_array();
        let kernel_root: Word = rand_array();
        let header = BlockHeader::mock(
            0,
            Some(chain_root.into()),
            Some(note_root.into()),
            &[],
            kernel_root.into(),
        );
        let serialized = header.to_bytes();
        let deserialized = BlockHeader::read_from_bytes(&serialized).unwrap();

        assert_eq!(deserialized, header);
    }
}
