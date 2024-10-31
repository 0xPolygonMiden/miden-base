use alloc::vec::Vec;
use std::string::ToString;

use assembly::{Assembler, Compile, Library};
use vm_processor::MastForest;

use crate::{accounts::StorageSlot, AccountError};

// TODO Document everything, add section separators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountComponent {
    pub(crate) code: Library,
    pub(crate) storage_slots: Vec<StorageSlot>,
    pub(crate) component_type: AccountComponentType,
}

impl AccountComponent {
    pub fn new(code: Library, storage_slots: Vec<StorageSlot>) -> Self {
        Self {
            code,
            storage_slots,
            component_type: AccountComponentType::Any,
        }
    }

    /// Returns a new [`AssembledAccountComponent`] compiled from the provided source code using the
    /// specified assembler.
    ///
    /// All procedures exported from the provided code will become members of the account's
    /// public interface.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Compilation of the provided source code fails.
    /// - The number of procedures exported from the provided library is 0 or greater than 256.
    pub fn compile(
        source_code: impl Compile,
        assembler: Assembler,
        storage_slots: Vec<StorageSlot>,
    ) -> Result<Self, AccountError> {
        let library = assembler
            .assemble_library([source_code])
            .map_err(|report| AccountError::AccountCodeAssemblyError(report.to_string()))?;

        Ok(Self::new(library, storage_slots))
    }

    pub fn with_type(mut self, component_type: AccountComponentType) -> Self {
        self.component_type = component_type;
        self
    }

    pub fn component_type(&self) -> AccountComponentType {
        self.component_type
    }

    pub fn is_faucet_component(&self) -> bool {
        self.component_type == AccountComponentType::Faucet
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AccountComponentType {
    Any,
    Faucet,
}
