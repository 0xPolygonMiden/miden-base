use vm_core::EMPTY_WORD;

use super::{
    AccountError, ByteReader, ByteWriter, Deserializable, DeserializationError, Felt, Serializable,
    Word,
};
use crate::{
    accounts::StorageMapDelta,
    crypto::{
        hash::rpo::RpoDigest,
        merkle::{InnerNodeInfo, LeafIndex, Smt, SmtLeaf, SmtProof, SMT_DEPTH},
    },
};

// ACCOUNT STORAGE MAP
// ================================================================================================
/// Empty storage map root.
pub const EMPTY_STORAGE_MAP_ROOT: Word = [
    Felt::new(15321474589252129342),
    Felt::new(17373224439259377994),
    Felt::new(15071539326562317628),
    Felt::new(3312677166725950353),
];

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

    /// The default value of empty leaves.
    pub const EMPTY_VALUE: Word = Smt::EMPTY_VALUE;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new [StorageMap].
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

    // DATA MUTATORS
    // --------------------------------------------------------------------------------------------
    pub fn insert(&mut self, key: RpoDigest, value: Word) -> Word {
        self.map.insert(key, value) // Delegate to Smt's insert method
    }

    /// Applies the provided delta to this account storage.
    ///
    /// This method assumes that the delta has been validated by the calling method and so, no
    /// additional validation of delta is performed.
    pub fn apply_delta(&mut self, delta: &StorageMapDelta) -> Result<(), AccountError> {
        // apply the updated leaves to the storage map
        for &(key, value) in delta.updated_leaves.iter() {
            self.set_map_item(key, value)?;
        }

        // apply the cleared leaves to the storage map
        // currently we cannot remove leaves from the storage map, so we just set them to empty
        for &key in delta.cleared_leaves.iter() {
            self.set_map_item(key, EMPTY_WORD)?;
        }

        Ok(())
    }

    /// Sets a map item from the storage at the specified index.
    pub fn set_map_item(&mut self, key: Word, value: Word) -> Result<(Word, Word), AccountError> {
        let old_map_root = self.root();
        let old_value = self.get_value(&RpoDigest::from(key));

        if value == EMPTY_WORD {
            // if the value is empty, remove the leaf from the storage map
            self.map.insert(key.into(), value);
        } else {
            // insert the value into the storage map
            self.map.insert(key.into(), value);
        }

        Ok((old_map_root.into(), old_value))
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
        self.map.write_into(target)
    }
}

impl Deserializable for StorageMap {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let smt = Smt::read_from(source)?;
        Ok(StorageMap { map: smt })
    }
}

#[cfg(test)]
mod tests {
    use miden_crypto::{hash::rpo::RpoDigest, Felt};

    use super::{Deserializable, Serializable, StorageMap, Word, EMPTY_STORAGE_MAP_ROOT};

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

    #[test]
    fn test_empty_storage_map_constants() {
        // If these values don't match, update the constants.
        assert_eq!(*StorageMap::default().root(), EMPTY_STORAGE_MAP_ROOT);
    }
}
