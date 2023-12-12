use super::{AccountError, AccountStorageDelta, Digest, Felt, Hasher, TryApplyDiff, Vec, Word};
use crate::crypto::merkle::{NodeIndex, SimpleSmt, StoreNode};

mod slot;
pub use slot::StorageSlotType;

// TYPE ALIASES
// ================================================================================================

/// A type that represents a single storage slot item. The tuple contains the slot index of the item
/// and the entry of the item.
pub type SlotItem = (u8, StorageSlot);

/// A type that represents a single storage slot entry. The tuple contains the type of the slot and
/// the value of the slot - the value can be a raw value or a commitment to the underlying data
/// structure.
pub type StorageSlot = (StorageSlotType, Word);

// ACCOUNT STORAGE
// ================================================================================================

/// Account storage consists of 256 index-addressable storage slots. Each slot has a type which
/// defines the size and the structure of the slot. Currently, the following types are supported:
/// - Scalar: a sequence of up to 256 words.
/// - Array: a sparse array of up to 2^n values where n < 64 and each value contains up to 256
///          words.
/// - Map: a key-value map where keys are words and values contain up to 256 words.
///
/// Storage slots are stored in a simple Sparse Merkle tree of depth 8. Slot 255 is always reserved
/// and contains information about slot types of all other slots.
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
        let mut types = vec![StorageSlotType::Value { value_arity: 0 }; 256];

        // set the slot type for the types commitment
        types[Self::SLOT_TYPES_COMMITMENT_INDEX as usize] =
            StorageSlotType::Value { value_arity: 64 };

        // process entries to extract type data
        let mut entires = items
            .into_iter()
            .map(|x| {
                if x.0 == 255 {
                    return Err(AccountError::StorageSlotIsReserved(x.0));
                }

                let (slot_type, slot_value) = x.1;
                types[x.0 as usize] = slot_type;
                Ok((x.0 as u64, slot_value))
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
