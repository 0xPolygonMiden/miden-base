use alloc::{collections::BTreeSet, string::ToString, sync::Arc, vec::Vec};

use assembly::{ast::Module, Assembler, Compile, Library, LibraryPath};
use vm_processor::MastForest;

mod template;
pub use template::*;

use crate::{
    account::{AccountType, StorageSlot},
    utils::{Deserializable, Serializable},
    AccountError, Digest,
};

/// An [`AccountComponent`] defines a [`Library`] of code and the initial value and types of
/// the [`StorageSlot`]s it accesses.
///
/// One or more components can be used to built [`AccountCode`](crate::account::AccountCode) and
/// [`AccountStorage`](crate::account::AccountStorage).
///
/// Each component is independent of other components and can only access its own storage slots.
/// Each component defines its own storage layout starting at index 0 up to the length of the
/// storage slots vector.
///
/// Components define the [`AccountType`]s they support, meaning whether the component can be used
/// to instantiate an account of that type. For example, a component implementing a fungible faucet
/// would only specify support for [`AccountType::FungibleFaucet`]. Using it to instantiate a
/// regular account would fail. By default, the set of supported types is empty, so each component
/// is forced to explicitly define what it supports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountComponent {
    pub(super) library: Library,
    pub(super) storage_slots: Vec<StorageSlot>,
    pub(super) supported_types: BTreeSet<AccountType>,
    pub(super) providing_interface: AccountInterfaceType,
}

impl AccountComponent {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new [`AccountComponent`] constructed from the provided `library` and
    /// `storage_slots`.
    ///
    /// All procedures exported from the provided code will become members of the account's public
    /// interface when added to an [`AccountCode`](crate::account::AccountCode).
    ///
    /// # Errors
    ///
    /// The following list of errors is exhaustive and can be relied upon for `expect`ing the call
    /// to this function. It is recommended that custom components ensure these conditions by design
    /// or in their fallible constructors.
    ///
    /// Returns an error if:
    /// - The number of given [`StorageSlot`]s exceeds 255.
    pub fn new(code: Library, storage_slots: Vec<StorageSlot>) -> Result<Self, AccountError> {
        // Check that we have less than 256 storage slots.
        u8::try_from(storage_slots.len())
            .map_err(|_| AccountError::StorageTooManySlots(storage_slots.len() as u64))?;

        let interface_type = AccountInterfaceType::Custom(*code.digest());

        Ok(Self {
            library: code,
            storage_slots,
            supported_types: BTreeSet::new(),
            providing_interface: interface_type,
        })
    }

    /// Returns a new [`AccountComponent`] whose library is compiled from the provided `source_code`
    /// using the specified `assembler` and with the given `storage_slots`.
    ///
    /// All procedures exported from the provided code will become members of the account's public
    /// interface when added to an [`AccountCode`](crate::account::AccountCode).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the compilation of the provided source code fails.
    /// - The number of storage slots exceeds 255.
    pub fn compile(
        source_code: impl Compile,
        assembler: Assembler,
        storage_slots: Vec<StorageSlot>,
    ) -> Result<Self, AccountError> {
        let library = assembler
            .assemble_library([source_code])
            .map_err(AccountError::AccountComponentAssemblyError)?;

        Self::new(library, storage_slots)
    }

    /// Returns a new [`AccountComponent`] whose library is compiled from the provided `source_code`
    /// using the specified `assembler`, `library_path`, and with the given `storage_slots`.
    ///
    /// All procedures exported from the provided code will become members of the account's public
    /// interface when added to an [`AccountCode`](crate::account::AccountCode), and could be called
    /// using the provided library path.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - the compilation of the provided source code fails.
    /// - The number of storage slots exceeds 255.
    pub fn compile_with_path(
        source_code: impl ToString,
        assembler: Assembler,
        storage_slots: Vec<StorageSlot>,
        library_path: LibraryPath,
    ) -> Result<Self, AccountError> {
        let source_manager = Arc::new(assembly::DefaultSourceManager::default());
        let module = Module::parser(assembly::ast::ModuleKind::Library)
            .parse_str(library_path, source_code, &source_manager)
            .map_err(AccountError::AccountComponentAssemblyError)?;

        let library = assembler
            .assemble_library(&[*module])
            .map_err(AccountError::AccountComponentAssemblyError)?;

        Self::new(library, storage_slots)
    }

    /// Instantiates an [AccountComponent] from the [AccountComponentTemplate].
    ///
    /// The template's component metadata might contain placeholders, which can be replaced by
    /// mapping storage placeholders to values through the `init_storage_data` parameter.
    ///
    /// # Errors
    ///
    /// - If any of the component's storage entries cannot be transformed into a valid storage slot.
    ///   This could be because the metadata is invalid, or storage values were not provided (or
    ///   they are not of a valid type)
    pub fn from_template(
        template: &AccountComponentTemplate,
        init_storage_data: &InitStorageData,
    ) -> Result<AccountComponent, AccountError> {
        let mut storage_slots = vec![];
        for storage_entry in template.metadata().storage_entries() {
            let entry_storage_slots = storage_entry
                .try_build_storage_slots(init_storage_data)
                .map_err(AccountError::AccountComponentTemplateInstantiationError)?;
            storage_slots.extend(entry_storage_slots);
        }

        Ok(AccountComponent::new(template.library().clone(), storage_slots)?
            .with_supported_types(template.metadata().supported_types().clone()))
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the number of storage slots accessible from this component.
    pub fn storage_size(&self) -> u8 {
        u8::try_from(self.storage_slots.len())
            .expect("storage slots len should fit in u8 per the constructor")
    }

    /// Returns a reference to the underlying [`Library`] of this component.
    pub fn library(&self) -> &Library {
        &self.library
    }

    /// Returns a reference to the underlying [`MastForest`] of this component.
    pub fn mast_forest(&self) -> &MastForest {
        self.library.mast_forest().as_ref()
    }

    /// Returns a slice of the underlying [`StorageSlot`]s of this component.
    pub fn storage_slots(&self) -> &[StorageSlot] {
        self.storage_slots.as_slice()
    }

    /// Returns a reference to the supported [`AccountType`]s.
    pub fn supported_types(&self) -> &BTreeSet<AccountType> {
        &self.supported_types
    }

    /// Returns `true` if this component supports the given `account_type`, `false` otherwise.
    pub fn supports_type(&self, account_type: AccountType) -> bool {
        self.supported_types.contains(&account_type)
    }

    pub fn providing_interface(&self) -> AccountInterfaceType {
        self.providing_interface
    }

    // MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Adds `supported_type` to the set of [`AccountType`]s supported by this component.
    ///
    /// This function has the semantics of [`BTreeSet::insert`], i.e. adding a type twice is fine
    /// and it can be called multiple times with different account types.
    pub fn with_supported_type(mut self, supported_type: AccountType) -> Self {
        self.supported_types.insert(supported_type);
        self
    }

    /// Overwrites any previously set supported types with the given set.
    ///
    /// This can be used to reset the supported types of a component to a chosen set, which may be
    /// useful after cloning an existing component.
    pub fn with_supported_types(mut self, supported_types: BTreeSet<AccountType>) -> Self {
        self.supported_types = supported_types;
        self
    }

    /// Sets the [`AccountType`]s supported by this component to all account types.
    pub fn with_supports_all_types(mut self) -> Self {
        self.supported_types.extend([
            AccountType::FungibleFaucet,
            AccountType::NonFungibleFaucet,
            AccountType::RegularAccountImmutableCode,
            AccountType::RegularAccountUpdatableCode,
        ]);
        self
    }

    pub fn with_account_interface(mut self, account_interface: AccountInterfaceType) -> Self {
        self.providing_interface = account_interface;
        self
    }
}

impl From<AccountComponent> for Library {
    fn from(component: AccountComponent) -> Self {
        component.library
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccountInterfaceType {
    BasicWallet,
    BasicFungibleFaucet,
    RpoFalcon512,
    // currently I'm using the library hash, but I think we should use the hash of the whole
    // component instead to be able to differentiate between two components with the same
    // libraries but different storage slots and supported types
    Custom(Digest),
}

impl Serializable for AccountInterfaceType {
    fn write_into<W: vm_core::utils::ByteWriter>(&self, target: &mut W) {
        match self {
            AccountInterfaceType::BasicWallet => target.write_u8(0),
            AccountInterfaceType::BasicFungibleFaucet => target.write_u8(1),
            AccountInterfaceType::RpoFalcon512 => target.write_u8(2),
            AccountInterfaceType::Custom(digest) => {
                target.write_u8(3);
                digest.write_into(target);
            },
        }
    }

    fn get_size_hint(&self) -> usize {
        match self {
            AccountInterfaceType::BasicWallet => 1,
            AccountInterfaceType::BasicFungibleFaucet => 1,
            AccountInterfaceType::RpoFalcon512 => 1,
            AccountInterfaceType::Custom(digest) => digest.get_size_hint() + 1,
        }
    }
}

impl Deserializable for AccountInterfaceType {
    fn read_from<R: vm_core::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, vm_processor::DeserializationError> {
        let type_index = source.read_u8()?;
        match type_index {
            0 => Ok(AccountInterfaceType::BasicWallet),
            1 => Ok(AccountInterfaceType::BasicFungibleFaucet),
            2 => Ok(AccountInterfaceType::RpoFalcon512),
            3 => {
                let library_commitment = Digest::read_from(source)?;
                Ok(AccountInterfaceType::Custom(library_commitment))
            },
            _ => Err(vm_processor::DeserializationError::InvalidValue(format!(
                "invalid account interface type: {type_index}"
            ))),
        }
    }
}
