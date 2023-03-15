use super::{Digest, Hasher, Word, ZERO};

// TYPE ALIASES
// ================================================================================================

pub type StorageItem = (Word, Word);

// ACCOUNT STORAGE
// ================================================================================================

/// Account storage is a key-value map where both keys and values are words (4 elements).
///
/// Internally, account storage uses a compact Sparse Merkle tree to store the data. To ensure that
/// the data is randomly distributed across the tree, the index of item in the tree is derived by
/// hashing the key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccountStorage {
    // TODO: add compact SMT as backing storage
}

impl AccountStorage {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new instance of account storage initialized with the provided items.
    pub fn new(_items: &[StorageItem]) -> Self {
        Self {}
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to this storage.
    pub fn root(&self) -> Digest {
        Digest::default()
    }

    /// Returns an item from the storage at the specified key. Both items and keys are Words which
    /// consist of 4 field elements.
    ///
    /// If the item is not present in the storage, [ZERO; 4] is returned.
    pub fn get_item(&self, key: Word) -> Word {
        // the index of the item is the hash of its key
        let _index: Word = Hasher::merge(&[key.into(), [ZERO; 4].into()]).into();
        todo!("retrieve the item from the tree");
    }

    /// Returns a list of items contained in this storage.
    pub fn items(&self) -> &[Word] {
        todo!()
    }
}
