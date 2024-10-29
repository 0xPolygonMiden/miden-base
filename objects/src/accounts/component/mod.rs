use alloc::vec::Vec;

use assembly::Library;
use vm_processor::MastForest;

use crate::accounts::StorageSlot;

// TODO Document everything, add section separators.
pub struct AccountComponent {
    pub(crate) code: Library,
    pub(crate) storage_slots: Vec<StorageSlot>,
}

impl AccountComponent {
    pub fn new(code: Library, storage_slots: Vec<StorageSlot>) -> Self {
        Self { code, storage_slots }
    }

    pub fn library(&self) -> &Library {
        &self.code
    }

    pub fn mast_forest(&self) -> &MastForest {
        self.code.mast_forest().as_ref()
    }

    pub fn storage_slots(&self) -> &[StorageSlot] {
        self.storage_slots.as_slice()
    }
}
