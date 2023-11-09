use super::{AccountError, AccountStorageDelta, Digest, Felt, Hasher, TryApplyDiff, Vec, Word};
use crate::crypto::merkle::{NodeIndex, SimpleSmt, StoreNode};

mod entry;
pub use entry::{StorageEntry, StorageEntryType};

mod slot;
pub use slot::{StorageSlot, StorageSlotType};

// TYPE ALIASES
// ================================================================================================

/// A type that represents a single storage slot item. The tuple contains the slot index of the item
/// and the entry of the item.
pub type SlotItem = (u8, StorageSlot);

// ACCOUNT STORAGE
// ================================================================================================

/// Account storage is composed of two components. The first component is a simple sparse Merkle
/// tree of depth 8 which is index addressable. This provides the user with 256 slots. The slots
/// can contain either a scalar, a map or an array entry. The second component is a vector of slot
/// types which describes the type of each slot. The slot types vector is committed to by
/// performing a sequential hash of the slot types vector.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct AccountStorage {
    slots: SimpleSmt,
    types: Vec<StorageSlotType>,
}

impl AccountStorage {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// Depth of the storage tree.
    pub const STORAGE_TREE_DEPTH: u8 = 8;

    /// The storage slot at which the slot types commitment is stored.
    pub const SLOT_TYPES_COMMITMENT_INDEX: u8 = 255;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new instance of account storage initialized with the provided items.
    pub fn new(items: Vec<SlotItem>) -> Result<AccountStorage, AccountError> {
        // initialize slot types vector
        let mut types = vec![StorageSlotType::Scalar(StorageEntryType::Scalar); 256];

        // set the slot type for the types commitment
        types[Self::SLOT_TYPES_COMMITMENT_INDEX as usize] =
            StorageSlotType::Scalar(StorageEntryType::Array { arity: 64 });

        // process entries to extract type data
        let mut entires = items
            .into_iter()
            .map(|x| {
                if x.0 == 255 {
                    return Err(AccountError::StorageSlotIsReserved(x.0));
                }

                let (slot_type, slot_entry) = x.1.into_inner();
                types[x.0 as usize] = slot_type;
                Ok((x.0 as u64, slot_entry.value()))
            })
            .collect::<Result<Vec<_>, AccountError>>()?;

        // add slot types commitment entry
        entires.push((
            Self::SLOT_TYPES_COMMITMENT_INDEX as u64,
            *Hasher::hash_elements(&types.iter().map(Felt::from).collect::<Vec<_>>()),
        ));

        // construct storage slots smt and populate the types vector.
        let slots = SimpleSmt::with_leaves(Self::STORAGE_TREE_DEPTH, entires)
            .map_err(AccountError::DuplicateStorageItems)?;

        // TODO: should we return a (Self, BTreeMap) tuple that includes the data for scalar slot
        // array entries?
        Ok(Self { slots, types })
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

    /// Returns a mutable reference to the sparse Merkle tree that backs the storage slots.
    pub fn slots_mut(&mut self) -> &mut SimpleSmt {
        &mut self.slots
    }

    /// Returns a slice of slot types.
    pub fn slot_types(&self) -> &[StorageSlotType] {
        &self.types
    }

    /// Returns a commitment to the storage slot types.
    pub fn slot_types_commitment(&self) -> Digest {
        Hasher::hash_elements(&self.types.iter().map(Felt::from).collect::<Vec<_>>())
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
}

impl TryApplyDiff<Digest, StoreNode> for AccountStorage {
    type DiffType = AccountStorageDelta;
    type Error = AccountError;

    fn try_apply(&mut self, diff: Self::DiffType) -> Result<(), Self::Error> {
        self.slots
            .try_apply(diff.slots_delta)
            .map_err(AccountError::ApplyStorageSlotsDiffFailed)?;
        Ok(())
    }
}
