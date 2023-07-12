use super::{AccountError, AccountStorageDelta, Digest, Vec, Word, EMPTY_WORD};
use crypto::{
    merkle::{MerkleStore, NodeIndex, SimpleSmt, StoreNode},
    utils::collections::{ApplyDiff, Diff},
};

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

    /// Returns a new instance of account storage initialized with the provided parts.
    pub fn from_parts(slots: SimpleSmt, store: MerkleStore) -> Self {
        Self { slots, store }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to this storage.
    pub fn root(&self) -> Digest {
        self.slots.root()
    }

    /// Returns an item from the storage at the specified index.
    ///
    /// If the item is not present in the storage, [ZERO; 4] is returned.
    pub fn get_item(&self, index: u8) -> Digest {
        let item_index = NodeIndex::new(Self::STORAGE_TREE_DEPTH, index as u64)
            .expect("index is u8 - index within range");
        self.slots.get_node(item_index).expect("index is u8 - index within range")
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

    // PUBLIC MODIFIERS
    // --------------------------------------------------------------------------------------------
    /// Sets an item from the storage at the specified index.
    pub fn set_item(&mut self, index: u8, value: Word) -> Word {
        self.slots
            .update_leaf(index as u64, value)
            .expect("index is u8 - index within range")
    }

    /// Sets the node, specified by the slot index and node index, to the specified value.
    pub fn set_store_node(
        &mut self,
        slot_index: u8,
        index: NodeIndex,
        value: Digest,
    ) -> Result<Digest, AccountError> {
        let root = self.get_item(slot_index);
        let root = self
            .store
            .set_node(root, index, value)
            .map_err(AccountError::SetStoreNodeFailed)?;
        self.set_item(slot_index, *root.root);
        Ok(root.root)
    }
}

impl Diff<Digest, StoreNode> for AccountStorage {
    type DiffType = AccountStorageDelta;

    fn diff(&self, other: &Self) -> AccountStorageDelta {
        if self.root() == other.root() {
            return AccountStorageDelta::default();
        }

        let mut cleared_slots = Vec::new();
        let mut updated_slots = Vec::new();
        let mut initial_slots = Vec::new();
        let mut final_slots = Vec::new();

        for idx in 0..2u64.pow(AccountStorage::STORAGE_TREE_DEPTH as u32) {
            let node_idx = NodeIndex::new(AccountStorage::STORAGE_TREE_DEPTH, idx).unwrap();

            let initial_value = self.get_item(node_idx.value() as u8);
            initial_slots.push(initial_value.into());

            let final_value = other.get_item(node_idx.value() as u8);
            final_slots.push(final_value.into());

            match initial_value == final_value {
                false if final_value == EMPTY_WORD.into() => cleared_slots.push(idx as u8),
                false => updated_slots.push((idx as u8, final_value.into())),
                true => (),
            }
        }
        let self_store = self.store().subset(initial_slots.iter());
        let other_store = other.store().subset(final_slots.iter());
        let store_delta = self_store.diff(&other_store);

        AccountStorageDelta {
            cleared_slots,
            updated_slots,
            store_delta,
        }
    }
}

impl ApplyDiff<Digest, StoreNode> for AccountStorage {
    type DiffType = AccountStorageDelta;

    fn apply(&mut self, diff: Self::DiffType) {
        for slot in diff.cleared_slots {
            self.set_item(slot, EMPTY_WORD.into());
        }
        for (slot, value) in diff.updated_slots {
            self.set_item(slot, value);
        }
        self.store.apply(diff.store_delta);
    }
}
