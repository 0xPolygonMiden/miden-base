use alloc::{string::String, vec::Vec};
use std::{collections::BTreeSet, string::ToString};

use assembly::Library;
use semver::Version;
use serde::{
    de::{Error as DeError, Unexpected},
    Deserialize, Deserializer, Serialize, Serializer,
};
use thiserror::Error;
use vm_core::utils::{Deserializable, Serializable};

use super::AccountType;

mod storage_entry;
pub use storage_entry::StorageEntry;

// COMPONENT PACKAGE
// ================================================================================================

/// Represents a package containing a component's metadata and its associated library.
///
/// The `ComponentPackage` encapsulates all necessary information to initialize and manage
/// a component within the system. It includes the configuration details and the compiled
/// library code required for the component's operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentPackage {
    /// The component's metadata. This describes the component and how the storage is laid out,
    /// alongside how storage values are initialized.
    metadata: ComponentMetadata,
    /// The account's previously-assembled code. This defines all functionality related to the
    /// component.
    library: Library,
}

impl ComponentPackage {
    /// Create a [ComponentPackage]
    pub fn new(
        metadata: ComponentMetadata,
        library: Library,
    ) -> Result<Self, ComponentMetadataError> {
        _ = toml::to_string(&metadata)
            .map_err(|err| ComponentMetadataError::MetadataDeserializationError(err.to_string()));
        Ok(Self { metadata, library })
    }

    pub fn metadata(&self) -> &ComponentMetadata {
        &self.metadata
    }

    pub fn library(&self) -> &Library {
        &self.library
    }
}

impl Serializable for ComponentPackage {
    fn write_into<W: vm_core::utils::ByteWriter>(&self, target: &mut W) {
        // Since `ComponentConfig::new` ensures valid TOML, unwrap is safe here.
        let config_toml =
            toml::to_string(&self.metadata).expect("Failed to serialize ComponentConfig to TOML");
        target.write(config_toml);
        target.write(&self.library);
    }
}

impl Deserializable for ComponentPackage {
    fn read_from<R: vm_core::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, vm_processor::DeserializationError> {
        // Read and deserialize the configuration from a TOML string.
        let config_str = String::read_from(source)?;
        let config: ComponentMetadata = toml::from_str(&config_str)
            .map_err(|e| vm_processor::DeserializationError::InvalidValue(e.to_string()))?;
        let library = Library::read_from(source)?;

        let package = ComponentPackage::new(config, library).map_err(|err| {
            vm_processor::DeserializationError::UnknownError(format!(
                "error deserializing into a ComponentPackage: {}",
                err
            ))
        })?;
        Ok(package)
    }
}

// COMPONENT METADATA
// ================================================================================================

/// Represents the full component template configuration.
///
/// This struct allows for serialization and deserialization to and from a TOML file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentMetadata {
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

impl ComponentMetadata {
    /// Create a new `ComponentMetadata`
    ///
    /// # Errors
    ///
    /// - If the specified storage slots are not contiguous across all storage entries
    pub fn new(
        name: String,
        description: String,
        version: Version,
        targets: BTreeSet<AccountType>,
        storage: Vec<StorageEntry>,
    ) -> Result<Self, ComponentMetadataError> {
        // Ensure no gaps in slots
        let mut all_slots: Vec<u8> =
            storage.iter().flat_map(|entry| entry.slot_indices().iter().copied()).collect();

        all_slots.sort_unstable();
        if let Some(v) = all_slots.get(0) {
            if *v != 0 {
                return Err(ComponentMetadataError::NonContiguousSlots);
            }
        }
        for slots in all_slots.windows(2) {
            if slots[1] != slots[0] + 1 {
                return Err(ComponentMetadataError::NonContiguousSlots);
            }
        }

        Ok(Self {
            name,
            description,
            version,
            targets,
            storage,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn targets(&self) -> &BTreeSet<AccountType> {
        &self.targets
    }

    pub fn storage(&self) -> &Vec<StorageEntry> {
        &self.storage
    }
}

#[derive(Debug, Error)]
pub enum ComponentMetadataError {
    #[error("component storage slots are not contiguous")]
    NonContiguousSlots,
    #[error("error deserializing component metadata: {0}")]
    MetadataDeserializationError(String),
}

// SERIALIZATION
// ================================================================================================

impl Serialize for AccountType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match self {
            AccountType::FungibleFaucet => "FungibleFaucet",
            AccountType::NonFungibleFaucet => "NonFungibleFaucet",
            AccountType::RegularAccountImmutableCode => "RegularAccountImmutableCode",
            AccountType::RegularAccountUpdatableCode => "RegularAccountUpdatableCode",
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for AccountType {
    fn deserialize<D>(deserializer: D) -> Result<AccountType, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;

        match s.to_lowercase().as_str() {
            "fungiblefaucet" => Ok(AccountType::FungibleFaucet),
            "nonfungiblefaucet" => Ok(AccountType::NonFungibleFaucet),
            "regularaccountimmutablecode" => Ok(AccountType::RegularAccountImmutableCode),
            "regularaccountupdatablecode" => Ok(AccountType::RegularAccountUpdatableCode),
            other => Err(D::Error::invalid_value(Unexpected::Str(other), &"a valid account type")),
        }
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use assembly::Assembler;
    use storage_entry::WordRepresentation;

    use super::*;
    use crate::testing::account_code::CODE;

    #[test]
    fn test_contiguous_value_slots() {
        let storage = vec![
            StorageEntry::Value {
                name: "slot0".into(),
                description: None,
                slot: 0,
                value: WordRepresentation::SingleHex(Default::default()),
            },
            StorageEntry::MultiSlot {
                name: "slot1".into(),
                description: None,
                slots: vec![1, 2],
                values: vec![
                    WordRepresentation::Array(Default::default()),
                    WordRepresentation::SingleHex(Default::default()),
                ],
            },
        ];

        let original_config = ComponentMetadata::new(
            "test".into(),
            "desc".into(),
            Version::parse("0.1.0").unwrap(),
            BTreeSet::new(),
            storage,
        )
        .unwrap();

        let serialized = toml::to_string(&original_config).unwrap();
        let deserialized: ComponentMetadata = toml::from_str(&serialized).unwrap();

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

        let result = ComponentMetadata::new(
            "test".into(),
            "desc".into(),
            Version::parse("0.1.0").unwrap(),
            BTreeSet::new(),
            storage,
        );
        assert!(result.is_err());
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
                    WordRepresentation::SingleHex(Default::default()),
                ],
            },
            StorageEntry::Value {
                name: "slot0".into(),
                description: None,
                slot: 0,
                value: WordRepresentation::SingleHex(Default::default()),
            },
        ];

        let component_template = ComponentMetadata::new(
            "test".into(),
            "desc".into(),
            Version::parse("0.1.0").unwrap(),
            BTreeSet::new(),
            storage,
        )
        .unwrap();

        let library = Assembler::default().assemble_library([CODE]).unwrap();
        let component = ComponentPackage::new(component_template, library).unwrap();

        let serialized = component.to_bytes();
        let deserialized = ComponentPackage::read_from_bytes(&serialized).unwrap();

        assert_eq!(deserialized, component)
    }
}
