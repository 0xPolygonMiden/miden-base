use alloc::{string::ToString, vec::Vec};

use super::{
    AccountError, ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable, Word,
};
use crate::crypto::{
    hash::rpo::RpoDigest,
    merkle::{InnerNodeInfo, LeafIndex, Smt, SmtLeaf, SmtProof, SMT_DEPTH},
};

// ACCOUNT STORAGE MAP
// ================================================================================================

/// Account storage map is a Sparse Merkle Tree of depth 64. It can be used to store more data as
/// there is in plain usage of the storage slots. The root of the SMT consumes one account storage
/// slot.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageMap {
    map: Smt,
}

impl StorageMap {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// Depth of the storage tree.
    pub const STORAGE_MAP_TREE_DEPTH: u8 = SMT_DEPTH;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new [AccountStorageMap].
    ///
    /// All leaves in the returned tree are set to [Self::EMPTY_VALUE].
    pub fn new() -> Self {
        StorageMap { map: Smt::new() }
    }

    pub fn with_entries(
        entries: impl IntoIterator<Item = (RpoDigest, Word)>,
    ) -> Result<Self, AccountError> {
        let mut storage_map = Smt::new();

        for (key, value) in entries {
            // Handle possible errors from insert, if applicable
            storage_map.insert(key, value);
        }

        Ok(StorageMap { map: storage_map })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    pub const fn depth(&self) -> u8 {
        SMT_DEPTH
    }

    pub fn root(&self) -> RpoDigest {
        self.map.root() // Delegate to Smt's root method
    }

    pub fn get_leaf(&self, key: &RpoDigest) -> SmtLeaf {
        self.map.get_leaf(key) // Delegate to Smt's get_leaf method
    }

    pub fn get_value(&self, key: &RpoDigest) -> Word {
        self.map.get_value(key) // Delegate to Smt's get_value method
    }

    pub fn open(&self, key: &RpoDigest) -> SmtProof {
        self.map.open(key) // Delegate to Smt's open method
    }

    // ITERATORS
    // --------------------------------------------------------------------------------------------
    pub fn leaves(&self) -> impl Iterator<Item = (LeafIndex<SMT_DEPTH>, &SmtLeaf)> {
        self.map.leaves() // Delegate to Smt's leaves method
    }

    pub fn entries(&self) -> impl Iterator<Item = &(RpoDigest, Word)> {
        self.map.entries() // Delegate to Smt's entries method
    }

    pub fn inner_nodes(&self) -> impl Iterator<Item = InnerNodeInfo> + '_ {
        self.map.inner_nodes() // Delegate to Smt's inner_nodes method
    }

    // STATE MUTATORS
    // --------------------------------------------------------------------------------------------

    pub fn insert(&mut self, key: RpoDigest, value: Word) -> Word {
        self.map.insert(key, value) // Delegate to Smt's insert method
    }
}

impl Default for StorageMap {
    fn default() -> Self {
        Self::new()
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for StorageMap {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        // Write the number of filled leaves for this StorageMap
        target.write_usize(self.entries().count());

        // Write each (key, value) pair
        for (key, value) in self.entries() {
            target.write(key);
            target.write(value);
        }
    }
}

impl Deserializable for StorageMap {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        // Read the number of filled leaves for this Smt
        let num_filled_leaves = source.read_usize()?;
        let mut entries = Vec::with_capacity(num_filled_leaves);

        for _ in 0..num_filled_leaves {
            let key = source.read()?;
            let value = source.read()?;
            entries.push((key, value));
        }

        Self::with_entries(entries)
            .map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use miden_crypto::{hash::rpo::RpoDigest, Felt};

    use super::{Deserializable, Serializable, StorageMap, Word};

    #[test]
    fn account_storage_serialization() {
        // StorageMap for default types (empty map)
        let storage_map_default = StorageMap::default();
        let bytes = storage_map_default.to_bytes();
        assert_eq!(storage_map_default, StorageMap::read_from_bytes(&bytes).unwrap());

        // StorageMap with values
        let storage_map_leaves_2: [(RpoDigest, Word); 2] = [
            (
                RpoDigest::new([Felt::new(101), Felt::new(102), Felt::new(103), Felt::new(104)]),
                [Felt::new(1_u64), Felt::new(2_u64), Felt::new(3_u64), Felt::new(4_u64)],
            ),
            (
                RpoDigest::new([Felt::new(105), Felt::new(106), Felt::new(107), Felt::new(108)]),
                [Felt::new(5_u64), Felt::new(6_u64), Felt::new(7_u64), Felt::new(8_u64)],
            ),
        ];
        let storage_map = StorageMap::with_entries(storage_map_leaves_2).unwrap();

        let bytes = storage_map.to_bytes();
        assert_eq!(storage_map, StorageMap::read_from_bytes(&bytes).unwrap());
    }
}
