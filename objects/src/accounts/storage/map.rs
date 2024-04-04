use super::{AccountError, Word};
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
