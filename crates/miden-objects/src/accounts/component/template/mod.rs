use alloc::{
    collections::BTreeSet,
    string::{String, ToString},
    vec::Vec,
};
use core::str::FromStr;

use assembly::Library;
use semver::Version;
use vm_core::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::DeserializationError;

use super::AccountType;
use crate::errors::AccountComponentTemplateError;

mod storage;
pub use storage::*;

// ACCOUNT COMPONENT TEMPLATE
// ================================================================================================

/// Represents a template containing a component's metadata and its associated library.
///
/// The [AccountComponentTemplate] encapsulates all necessary information to initialize and manage
/// an account component within the system. It includes the configuration details and the compiled
/// library code required for the component's operation.
///
/// A template can be instantiated into [AccountComponent](super::AccountComponent) objects.
/// The component metadata can be defined with placeholders that can be replaced at instantiation
/// time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountComponentTemplate {
    /// The component's metadata. This describes the component and how the storage is laid out,
    /// alongside how storage values are initialized.
    metadata: AccountComponentMetadata,
    /// The account component's assembled code. This defines all functionality related to the
    /// component.
    library: Library,
}

impl AccountComponentTemplate {
    /// Creates a new [AccountComponentTemplate].
    ///
    /// This template holds everything needed to describe and implement a component, including the
    /// compiled procedures (via the [Library]) and the metadata that defines the component’s
    /// storage layout ([AccountComponentMetadata]). The metadata can include storage placeholders
    /// that get filled in at the time of the [AccountComponent](super::AccountComponent)
    /// instantiation.
    pub fn new(metadata: AccountComponentMetadata, library: Library) -> Self {
        Self { metadata, library }
    }

    /// Returns a reference to the template's [AccountComponentMetadata].
    pub fn metadata(&self) -> &AccountComponentMetadata {
        &self.metadata
    }

    /// Returns a reference to the underlying [Library] of this component.
    pub fn library(&self) -> &Library {
        &self.library
    }
}

impl Serializable for AccountComponentTemplate {
    fn write_into<W: vm_core::utils::ByteWriter>(&self, target: &mut W) {
        target.write(&self.metadata);
        target.write(&self.library);
    }
}

impl Deserializable for AccountComponentTemplate {
    fn read_from<R: vm_core::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, vm_processor::DeserializationError> {
        // Read and deserialize the configuration from a TOML string.
        let metadata: AccountComponentMetadata = source.read()?;
        let library = Library::read_from(source)?;

        Ok(AccountComponentTemplate::new(metadata, library))
    }
}

// ACCOUNT COMPONENT METADATA
// ================================================================================================

/// Represents the full component template configuration.
///
/// An account component metadata describes the component alongside its storage layout.
/// On the storage layout, [placeholders](StoragePlaceholder) can be utilized to identify
/// [values](StorageValue) that should be provided at the moment of instantiation.
///
/// When the `std` feature is enabled, this struct allows for serialization and deserialization to
/// and from a TOML file.
///
/// # Example
///
/// ```
/// # use semver::Version;
/// # use std::collections::BTreeSet;
/// # use miden_objects::{testing::account_code::CODE, accounts::{
/// #     AccountComponent, AccountComponentMetadata, InitStorageData, StorageEntry,
/// #     StoragePlaceholder, StorageValue,
/// #     AccountComponentTemplate, FeltRepresentation, WordRepresentation},
/// #     assembly::Assembler, Felt};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let first_felt = FeltRepresentation::Decimal(Felt::new(0u64));
/// let second_felt = FeltRepresentation::Decimal(Felt::new(1u64));
/// let third_felt = FeltRepresentation::Decimal(Felt::new(2u64));
/// // Templated element:
/// let last_element = FeltRepresentation::Template(StoragePlaceholder::new("foo")?);
///
/// let storage_entry = StorageEntry::new_value(
///     "test-entry",
///     Some("a test entry"),
///     0,
///     WordRepresentation::Array([first_felt, second_felt, third_felt, last_element]),
/// );
///
/// let init_storage_data = InitStorageData::new([(
///     StoragePlaceholder::new("foo")?,
///     StorageValue::Felt(Felt::new(300u64)),
/// )]);
///
/// let component_template = AccountComponentMetadata::new(
///     "test".into(),
///     "desc".into(),
///     Version::parse("0.1.0")?,
///     BTreeSet::new(),
///     vec![],
/// )?;
///
/// let library = Assembler::default().assemble_library([CODE]).unwrap();
/// let template = AccountComponentTemplate::new(component_template, library);
///
/// let component = AccountComponent::from_template(&template, &init_storage_data)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct AccountComponentMetadata {
    /// The human-readable name of the component.
    name: String,

    /// A brief description of what this component is and how it works.
    description: String,

    /// The version of the component using semantic versioning.
    /// This can be used to track and manage component upgrades.
    version: Version,

    /// A set of supported target account types for this component.
    targets: BTreeSet<AccountType>,

    /// A list of storage entries defining the component's storage layout and initialization
    /// values.
    storage: Vec<StorageEntry>,
}

impl AccountComponentMetadata {
    /// Create a new [AccountComponentMetadata].
    ///
    /// # Errors
    ///
    /// - If the specified storage slots contain duplicates.
    /// - If the slot numbers do not start at zero.
    /// - If the slots are not contiguous.
    pub fn new(
        name: String,
        description: String,
        version: Version,
        targets: BTreeSet<AccountType>,
        storage: Vec<StorageEntry>,
    ) -> Result<Self, AccountComponentTemplateError> {
        let component = Self {
            name,
            description,
            version,
            targets,
            storage,
        };
        component.validate()?;
        Ok(component)
    }

    /// Retrieves the set of storage placeholder keys (identified by a string) that
    /// require a value at the moment of component instantiation. These values will
    /// be used for initializing storage slot values, or storage map entries.
    /// For a full example on how a placeholder may be utilized, refer to the docs
    /// for [AccountComponentMetadata].
    pub fn get_storage_placeholders(&self) -> BTreeSet<StoragePlaceholder> {
        let mut placeholder_set = BTreeSet::new();
        for storage_entry in &self.storage {
            for key in storage_entry.placeholders() {
                placeholder_set.insert(key.clone());
            }
        }
        placeholder_set
    }

    /// Returns the name of the account component.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the description of the account component.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the semantic version of the account component.
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Returns the account types supported by the component.
    pub fn targets(&self) -> &BTreeSet<AccountType> {
        &self.targets
    }

    /// Returns the list of storage entries of the component.
    pub fn storage_entries(&self) -> &Vec<StorageEntry> {
        &self.storage
    }

    /// Validate the [AccountComponentMetadata].
    ///
    /// # Errors
    ///
    /// - If the specified storage slots contain duplicates.
    /// - If the slot numbers do not start at zero.
    /// - If the slots are not contiguous.
    fn validate(&self) -> Result<(), AccountComponentTemplateError> {
        let mut all_slots: Vec<u8> = self
            .storage
            .iter()
            .flat_map(|entry| entry.slot_indices().iter().copied())
            .collect();

        // Check that slots start at 0 and are contiguous
        all_slots.sort_unstable();
        if let Some(&first_slot) = all_slots.first() {
            if first_slot != 0 {
                return Err(AccountComponentTemplateError::StorageSlotsMustStartAtZero);
            }
        }

        for slots in all_slots.windows(2) {
            if slots[1] == slots[0] {
                return Err(AccountComponentTemplateError::DuplicateSlot(slots[0]));
            }

            if slots[1] != slots[0] + 1 {
                return Err(AccountComponentTemplateError::NonContiguousSlots(slots[0], slots[1]));
            }
        }
        Ok(())
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountComponentMetadata {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.name.write_into(target);
        self.description.write_into(target);
        self.version.to_string().write_into(target);
        self.targets.write_into(target);
        self.storage.write_into(target);
    }
}

impl Deserializable for AccountComponentMetadata {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        Ok(Self {
            name: String::read_from(source)?,
            description: String::read_from(source)?,
            version: semver::Version::from_str(&String::read_from(source)?).map_err(
                |err: semver::Error| DeserializationError::InvalidValue(err.to_string()),
            )?,
            targets: BTreeSet::<AccountType>::read_from(source)?,
            storage: Vec::<StorageEntry>::read_from(source)?,
        })
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {

    use assembly::Assembler;
    use assert_matches::assert_matches;
    use storage::WordRepresentation;

    use super::*;
    use crate::{accounts::AccountComponent, testing::account_code::CODE};

    #[test]
    fn test_contiguous_value_slots() {
        let storage = vec![
            StorageEntry::Value {
                name: "slot0".into(),
                description: None,
                slot: 0,
                value: WordRepresentation::Hexadecimal(Default::default()),
            },
            StorageEntry::MultiSlot {
                name: "slot1".into(),
                description: None,
                slots: vec![1, 2],
                values: vec![
                    WordRepresentation::Array(Default::default()),
                    WordRepresentation::Hexadecimal(Default::default()),
                ],
            },
        ];

        let original_config = AccountComponentMetadata::new(
            "test".into(),
            "desc".into(),
            Version::parse("0.1.0").unwrap(),
            BTreeSet::new(),
            storage,
        )
        .unwrap();

        let serialized = original_config.as_toml().unwrap();

        let deserialized = AccountComponentMetadata::from_toml(&serialized).unwrap();
        assert_eq!(deserialized, original_config)
    }

    #[test]
    fn test_new_non_contiguous_value_slots() {
        let storage = vec![
            StorageEntry::Value {
                name: "slot0".into(),
                description: None,
                slot: 0,
                value: Default::default(),
            },
            StorageEntry::Value {
                name: "slot2".into(),
                description: None,
                slot: 2,
                value: Default::default(),
            },
        ];

        let result = AccountComponentMetadata::new(
            "test".into(),
            "desc".into(),
            Version::parse("0.1.0").unwrap(),
            BTreeSet::new(),
            storage,
        );
        assert_matches!(result, Err(AccountComponentTemplateError::NonContiguousSlots(0, 2)));
    }

    #[test]
    fn test_binary_serde_roundtrip() {
        let storage = vec![
            StorageEntry::MultiSlot {
                name: "slot1".into(),
                description: None,
                slots: vec![1, 2],
                values: vec![
                    WordRepresentation::Array(Default::default()),
                    WordRepresentation::Hexadecimal(Default::default()),
                ],
            },
            StorageEntry::Value {
                name: "slot0".into(),
                description: None,
                slot: 0,
                value: WordRepresentation::Hexadecimal(Default::default()),
            },
        ];

        let component_template = AccountComponentMetadata::new(
            "test".into(),
            "desc".into(),
            Version::parse("0.1.0").unwrap(),
            BTreeSet::new(),
            storage,
        )
        .unwrap();

        let library = Assembler::default().assemble_library([CODE]).unwrap();
        let template = AccountComponentTemplate::new(component_template, library);
        _ = AccountComponent::from_template(&template, &InitStorageData::default()).unwrap();

        let serialized = template.to_bytes();
        let deserialized = AccountComponentTemplate::read_from_bytes(&serialized).unwrap();

        assert_eq!(deserialized, template)
    }
}
