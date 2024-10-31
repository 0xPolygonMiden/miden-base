use alloc::{collections::BTreeSet, vec::Vec};
use std::string::ToString;

use assembly::{Assembler, Compile, Library};
use vm_processor::MastForest;

use crate::{
    accounts::{AccountType, StorageSlot},
    AccountError,
};

// TODO Document everything, add section separators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountComponent {
    pub(crate) library: Library,
    pub(crate) storage_slots: Vec<StorageSlot>,
    pub(crate) supported_types: BTreeSet<AccountType>,
}

impl AccountComponent {
    pub fn new(code: Library, storage_slots: Vec<StorageSlot>) -> Self {
        Self {
            library: code,
            storage_slots,
            supported_types: BTreeSet::new(),
        }
    }

    /// Returns a new [`AccountComponent`] compiled from the provided source code using the
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

    pub fn with_supported_type(mut self, supported_type: AccountType) -> Self {
        self.supported_types.insert(supported_type);
        self
    }

    pub fn with_supported_types(mut self, supported_types: BTreeSet<AccountType>) -> Self {
        self.supported_types = supported_types;
        self
    }

    pub fn with_supports_all_types(mut self) -> Self {
        self.supported_types.extend([
            AccountType::FungibleFaucet,
            AccountType::NonFungibleFaucet,
            AccountType::RegularAccountImmutableCode,
            AccountType::RegularAccountUpdatableCode,
        ]);
        self
    }

    pub fn supported_types(&self) -> &BTreeSet<AccountType> {
        &self.supported_types
    }

    pub fn library(&self) -> &Library {
        &self.library
    }

    pub fn mast_forest(&self) -> &MastForest {
        self.library.mast_forest().as_ref()
    }

    pub fn storage_slots(&self) -> &[StorageSlot] {
        self.storage_slots.as_slice()
    }
}

impl From<AccountComponent> for Library {
    fn from(component: AccountComponent) -> Self {
        component.library
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AccountComponentType {
    Any,
    Faucet,
}
