use super::{AccountError, Word};
use crypto::merkle::{MerkleStore, NodeIndex, SimpleSmt};

// TYPE ALIASES
// ================================================================================================

pub type StorageItem = (u8, Word);

// ACCOUNT STORAGE
// ================================================================================================

/// Account storage is composed of two components. The first component is a simple sparse Merkle
/// tree of depth 8 which is index addressable. This provides the user with 256 Word slots. Users
/// that require additional storage can use the second component which is a `MerkleStore`. This
/// will allow the user to store any Merkle structures they need.  This is achieved by storing the
/// root of the Merkle structure as a leaf in the simple sparse merkle tree. When `AccountStorage`
/// is serialized it will check to see if any of the leafs in the simple sparse Merkle tree are
/// Merkle roots of other Merkle structures.  If any Merkle roots are found then the Merkle
/// structures will be persisted in the `AccountStorage` `MerkleStore`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountStorage {
    slots: SimpleSmt,
    store: MerkleStore,
}

impl AccountStorage {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// Depth of the storage tree.
    pub const STORAGE_TREE_DEPTH: u8 = 8;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new instance of account storage initialized with the provided items.
    pub fn new(
        items: Vec<StorageItem>,
        store: MerkleStore,
    ) -> Result<AccountStorage, AccountError> {
        // construct storage slots smt
        let slots = SimpleSmt::with_leaves(
            Self::STORAGE_TREE_DEPTH,
            items.into_iter().map(|x| (x.0 as u64, x.1)),
        )
        .map_err(AccountError::DuplicateStorageItems)?;
        Ok(Self { slots, store })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to this storage.
    pub fn root(&self) -> Word {
        self.slots.root()
    }

    /// Returns an item from the storage at the specified index.
    ///
    /// If the item is not present in the storage, [ZERO; 4] is returned.
    pub fn get_item(&self, index: u8) -> Word {
        let item_index = NodeIndex::new(Self::STORAGE_TREE_DEPTH, index as u64)
            .expect("index is u8 - index within range");
        self.slots.get_node(item_index).expect("index is u8 - index within range")
    }

    /// Sets an item from the storage at the specified index.
    pub fn set_item(&mut self, index: u8, value: Word) -> Word {
        self.slots
            .update_leaf(index as u64, value)
            .expect("index is u8 - index within range")
    }

    /// Returns a reference to the sparse Merkle tree that backs the storage slots.
    pub fn slots(&self) -> &SimpleSmt {
        &self.slots
    }

    /// Returns a reference to the Merkle store that backs the storage.
    pub fn store(&self) -> &MerkleStore {
        &self.store
    }

    /// Returns a list of items contained in this storage.
    pub fn items(&self) -> &[Word] {
        todo!()
    }
}
