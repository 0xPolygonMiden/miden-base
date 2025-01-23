use alloc::{boxed::Box, string::String, vec::Vec};

use vm_core::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::DeserializationError;

mod entry_content;
pub use entry_content::*;

use super::AccountComponentTemplateError;
use crate::account::StorageSlot;

mod placeholder;
pub use placeholder::{PlaceholderType, StoragePlaceholder, StorageValue};

mod init_storage_data;
pub use init_storage_data::InitStorageData;

#[cfg(feature = "std")]
pub mod toml;

// STORAGE ENTRY
// ================================================================================================

/// Represents a single entry in the component's storage layout.
///
/// Each entry can describe:
/// - A value slot with a single word.
/// - A map slot with a key-value map that occupies one storage slot.
/// - A multi-slot entry spanning multiple contiguous slots with multiple words (but not maps) that
///   represent a single logical value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageEntry {
    /// A value slot, which can contain one or more words. Each word is a hex-encoded string.
    Value {
        /// The human-readable name of the slot.
        name: String,
        /// An optional description for the slot, explaining its purpose.
        description: Option<String>,
        /// The numeric index of this slot in the component's storage layout.
        slot: u8,
        /// The initial value for this slot.
        value: WordRepresentation,
    },

    /// A map slot, containing multiple key-value pairs. Keys and values are hex-encoded strings.
    Map {
        /// The human-readable name of the map slot.
        name: String,
        /// An optional description for the slot, explaining its purpose.
        description: Option<String>,
        /// The numeric index of this map slot in the component's storage.
        slot: u8,
        /// A list of key-value pairs to initialize in this map slot.
        map: MapRepresentation,
    },

    /// A multi-slot entry, representing a single logical value across multiple slots.
    MultiSlot {
        /// The human-readable name of this multi-slot entry.
        name: String,
        /// An optional description for the slot, explaining its purpose.
        description: Option<String>,
        /// The indices of the slots that form this multi-slot entry.
        slots: Vec<u8>,
        /// A list of values to fill the logical slot, with a length equal to the amount of slots.
        values: Vec<WordRepresentation>,
    },
}

impl StorageEntry {
    /// Creates a new [`StorageEntry::Value`] variant.
    pub fn new_value(
        name: impl Into<String>,
        description: Option<impl Into<String>>,
        slot: u8,
        value: impl Into<WordRepresentation>,
    ) -> Self {
        StorageEntry::Value {
            name: name.into(),
            description: description.map(Into::<String>::into),
            slot,
            value: value.into(),
        }
    }

    /// Creates a new [`StorageEntry::Map`] variant.
    pub fn new_map(
        name: impl Into<String>,
        description: Option<impl Into<String>>,
        slot: u8,
        map_representation: MapRepresentation,
    ) -> Result<Self, AccountComponentTemplateError> {
        let entry = StorageEntry::Map {
            name: name.into(),
            description: description.map(Into::<String>::into),
            slot,
            map: map_representation,
        };

        entry.validate()?;
        Ok(entry)
    }

    /// Creates a new [`StorageEntry::MultiSlot`] variant.
    pub fn new_multi_slot(
        name: impl Into<String>,
        description: Option<impl Into<String>>,
        slots: Vec<u8>,
        values: Vec<impl Into<WordRepresentation>>,
    ) -> Result<Self, AccountComponentTemplateError> {
        let entry = StorageEntry::MultiSlot {
            name: name.into(),
            description: description.map(Into::<String>::into),
            slots,
            values: values.into_iter().map(Into::into).collect(),
        };

        entry.validate()?;
        Ok(entry)
    }

    /// Returns the slot indices that the storage entry covers.
    pub fn slot_indices(&self) -> &[u8] {
        match self {
            StorageEntry::MultiSlot { slots, .. } => slots.as_slice(),
            StorageEntry::Value { slot, .. } => core::slice::from_ref(slot),
            StorageEntry::Map { slot, .. } => core::slice::from_ref(slot),
        }
    }

    /// Returns the name of the storage entry.
    pub fn name(&self) -> &str {
        match self {
            StorageEntry::Value { name, .. } => name.as_str(),
            StorageEntry::Map { name, .. } => name.as_str(),
            StorageEntry::MultiSlot { name, .. } => name.as_str(),
        }
    }

    /// Returns the optional description of the storage entry.
    pub fn description(&self) -> Option<&str> {
        match self {
            StorageEntry::Value { description, .. } => description.as_deref(),
            StorageEntry::Map { description, .. } => description.as_deref(),
            StorageEntry::MultiSlot { description, .. } => description.as_deref(),
        }
    }

    /// Returns all the `WordRepresentation` values covered by this entry.
    /// For `Value` entries, this returns a single-element slice.
    /// For `MultiSlot` entries, this returns all values.
    /// For `Map` entries, since they're key-value pairs, return an empty slice.
    pub fn word_values(&self) -> &[WordRepresentation] {
        match self {
            StorageEntry::Value { value, .. } => core::slice::from_ref(value),
            StorageEntry::MultiSlot { values, .. } => values.as_slice(),
            StorageEntry::Map { .. } => &[],
        }
    }

    /// Returns an iterator over all of the storage entries's placeholder keys, alongside their
    /// expected type.
    pub fn all_placeholders_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (&StoragePlaceholder, PlaceholderType)> + '_> {
        match self {
            StorageEntry::Value { value, .. } => value.all_placeholders_iter(),
            StorageEntry::Map { map: map_entries, .. } => map_entries.all_placeholders_iter(),
            StorageEntry::MultiSlot { values, .. } => {
                Box::new(values.iter().flat_map(|word| word.all_placeholders_iter()))
            },
        }
    }

    /// Attempts to convert the storage entry into a list of [StorageSlot].
    ///
    /// - StorageEntry::Value would convert to a [StorageSlot::Value]
    /// - StorageEntry::MultiSlot would convert to as many [StorageSlot::Value] as defined
    /// - StorageEntry::Map would convert to a [StorageSlot::Map]
    ///
    /// Each of the entry's values could be templated. These values are replaced for values found
    /// in `init_storage_data`, identified by its key.
    pub fn try_build_storage_slots(
        &self,
        init_storage_data: &InitStorageData,
    ) -> Result<Vec<StorageSlot>, AccountComponentTemplateError> {
        match self {
            StorageEntry::Value { value, .. } => {
                let slot = value.try_build_word(init_storage_data)?;
                Ok(vec![StorageSlot::Value(slot)])
            },
            StorageEntry::Map { map: values, .. } => {
                let storage_map = values.try_build_map(init_storage_data)?;
                Ok(vec![StorageSlot::Map(storage_map)])
            },
            StorageEntry::MultiSlot { values, .. } => Ok(values
                .iter()
                .map(|word_repr| {
                    word_repr.clone().try_build_word(init_storage_data).map(StorageSlot::Value)
                })
                .collect::<Result<Vec<StorageSlot>, _>>()?),
        }
    }

    /// Validates the storage entry for internal consistency.
    pub(super) fn validate(&self) -> Result<(), AccountComponentTemplateError> {
        match self {
            StorageEntry::Map { map, .. } => map.validate(),
            StorageEntry::MultiSlot { slots, values, .. } => {
                if slots.len() != values.len() {
                    return Err(AccountComponentTemplateError::MultiSlotArityMismatch);
                } else {
                    let mut all_slots = slots.clone();
                    all_slots.sort_unstable();
                    for slots in all_slots.windows(2) {
                        if slots[1] == slots[0] {
                            return Err(AccountComponentTemplateError::DuplicateSlot(slots[0]));
                        }

                        if slots[1] != slots[0] + 1 {
                            return Err(AccountComponentTemplateError::NonContiguousSlots(
                                slots[0], slots[1],
                            ));
                        }
                    }
                }
                Ok(())
            },
            StorageEntry::Value { .. } => Ok(()),
        }
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for StorageEntry {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            StorageEntry::Value { name, description, slot, value } => {
                target.write_u8(0u8);
                target.write(name);
                target.write(description);
                target.write_u8(*slot);
                target.write(value);
            },
            StorageEntry::Map { name, description, slot, map: values } => {
                target.write_u8(1u8);
                target.write(name);
                target.write(description);
                target.write_u8(*slot);
                target.write(values);
            },
            StorageEntry::MultiSlot { name, description, slots, values } => {
                target.write_u8(2u8);
                target.write(name);
                target.write(description);
                target.write(slots);
                target.write(values);
            },
        }
    }
}

impl Deserializable for StorageEntry {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let variant_tag = source.read_u8()?;
        let name: String = source.read()?;
        let description: Option<String> = source.read()?;

        match variant_tag {
            // Value
            0 => {
                let slot = source.read_u8()?;
                let value: WordRepresentation = source.read()?;

                Ok(StorageEntry::Value { name, description, slot, value })
            },

            // Map
            1 => {
                let slot = source.read_u8()?;
                let map_representation: MapRepresentation = source.read()?;

                Ok(StorageEntry::Map {
                    name,
                    description,
                    slot,
                    map: map_representation,
                })
            },

            // MultiSlot
            2 => {
                let slots: Vec<u8> = source.read()?;
                let values: Vec<WordRepresentation> = source.read()?;

                Ok(StorageEntry::MultiSlot { name, description, slots, values })
            },

            // Unknown tag => error
            _ => Err(DeserializationError::InvalidValue(format!(
                "unknown variant tag `{variant_tag}` for StorageEntry"
            ))),
        }
    }
}

// MAP ENTRY
// ================================================================================================

/// Key-value entry for storage maps.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct MapEntry {
    key: WordRepresentation,
    value: WordRepresentation,
}

impl MapEntry {
    pub fn new(key: impl Into<WordRepresentation>, value: impl Into<WordRepresentation>) -> Self {
        Self { key: key.into(), value: value.into() }
    }

    pub fn key(&self) -> &WordRepresentation {
        &self.key
    }

    pub fn value(&self) -> &WordRepresentation {
        &self.value
    }

    /// Returns an iterator over all of the storage entries's placeholder keys, alongside their
    /// expected type.
    pub fn all_placeholders_iter(
        &self,
    ) -> impl Iterator<Item = (&StoragePlaceholder, PlaceholderType)> {
        self.key.all_placeholders_iter().chain(self.value.all_placeholders_iter())
    }

    pub fn into_parts(self) -> (WordRepresentation, WordRepresentation) {
        let MapEntry { key, value } = self;
        (key, value)
    }
}

impl Serializable for MapEntry {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.key.write_into(target);
        self.value.write_into(target);
    }
}

impl Deserializable for MapEntry {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let key = WordRepresentation::read_from(source)?;
        let value = WordRepresentation::read_from(source)?;
        Ok(MapEntry { key, value })
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use core::panic;
    use std::collections::BTreeSet;

    use assembly::Assembler;
    use assert_matches::assert_matches;
    use semver::Version;
    use vm_core::{Felt, FieldElement};

    use super::*;
    use crate::{
        account::{
            component::template::{AccountComponentMetadata, AccountComponentTemplate},
            AccountComponent, AccountType, StorageMap,
        },
        digest,
        testing::account_code::CODE,
        AccountError,
    };

    #[test]
    fn test_storage_entry_serialization() {
        let array = [
            FeltRepresentation::Decimal(Felt::new(0xabc)),
            FeltRepresentation::Decimal(Felt::new(1218)),
            FeltRepresentation::Hexadecimal(Felt::new(0xdba3)),
            FeltRepresentation::Template(StoragePlaceholder::new("test.array.dyn").unwrap()),
        ];
        let storage = vec![
            StorageEntry::Value {
                name: "slot0".into(),
                description: Some("First slot".into()),
                slot: 0,
                value: WordRepresentation::Value(digest!("0x333123").into()),
            },
            StorageEntry::Map {
                name: "map".into(),
                description: Some("A storage map entry".into()),
                slot: 1,
                map: MapRepresentation::List(vec![
                    MapEntry {
                        key: WordRepresentation::Template(
                            StoragePlaceholder::new("foo.bar").unwrap(),
                        ),
                        value: WordRepresentation::Value(digest!("0x2").into()),
                    },
                    MapEntry {
                        key: WordRepresentation::Value(digest!("0x2").into()),
                        value: WordRepresentation::Template(
                            StoragePlaceholder::new("bar.baz").unwrap(),
                        ),
                    },
                    MapEntry {
                        key: WordRepresentation::Value(digest!("0x3").into()),
                        value: WordRepresentation::Value(digest!("0x4").into()),
                    },
                ]),
            },
            StorageEntry::MultiSlot {
                name: "multi".into(),
                description: Some("Multi slot entry".into()),
                slots: vec![2, 3, 4],
                values: vec![
                    WordRepresentation::Template(StoragePlaceholder::new("test.Template").unwrap()),
                    WordRepresentation::Array(array),
                    WordRepresentation::Value(digest!("0xabcdef123abcdef123").into()),
                ],
            },
            StorageEntry::Value {
                name: "single-slot".into(),
                description: Some("Slot with storage placeholder".into()),
                slot: 5,
                value: WordRepresentation::Template(
                    StoragePlaceholder::new("single-slot-key").unwrap(),
                ),
            },
        ];

        let config = AccountComponentMetadata {
            name: "Test Component".into(),
            description: "This is a test component".into(),
            version: Version::parse("1.0.0").unwrap(),
            targets: BTreeSet::from([AccountType::FungibleFaucet]),
            storage,
        };

        let toml = config.as_toml().unwrap();

        let deserialized = AccountComponentMetadata::from_toml(&toml).unwrap();

        assert_eq!(deserialized, config);
    }

    #[test]
    pub fn test_toml() {
        let toml_text = r#"
            name = "Test Component"
            description = "This is a test component"
            version = "1.0.1"
            targets = ["FungibleFaucet"]

            [[storage]]
            name = "map"
            description = "A storage map entry"
            slot = 0
            values = [
                { key = "0x1", value = ["{{value.test}}", "0x1", "0x2", "0x3"] },
                { key = "{{map.key.test}}", value = "0x3" },
                { key = "0x3", value = "0x4" }
            ]

            [[storage]]
            name = "test-word"
            description = "word"
            slot = 1
            value = "{{word.test}}" 

            [[storage]]
            name = "multitest"
            description = "a multi slot test"
            slots = [2, 3]
            values = [
                "{{word.test}}",
                ["1", "0", "0", "0"],
            ]

            [[storage]]
            name = "map-template"
            description = "a templated map"
            slot = 4
            values = "{{map.template}}"
        "#;

        let component_metadata = AccountComponentMetadata::from_toml(toml_text).unwrap();
        for (key, placeholder_type) in component_metadata.get_unique_storage_placeholders() {
            match key.inner() {
                "map.key.test" | "word.test" => assert_eq!(placeholder_type, PlaceholderType::Word),
                "value.test" => assert_eq!(placeholder_type, PlaceholderType::Felt),
                "map.template" => assert_eq!(placeholder_type, PlaceholderType::Map),
                _ => panic!("all cases are covered"),
            }
        }

        let library = Assembler::default().assemble_library([CODE]).unwrap();

        let template = AccountComponentTemplate::new(component_metadata, library);

        let template_bytes = template.to_bytes();
        let template_deserialized =
            AccountComponentTemplate::read_from_bytes(&template_bytes).unwrap();
        assert_eq!(template, template_deserialized);

        let storage_placeholders = InitStorageData::new([
            (
                StoragePlaceholder::new("map.key.test").unwrap(),
                StorageValue::Word(Default::default()),
            ),
            (
                StoragePlaceholder::new("value.test").unwrap(),
                StorageValue::Felt(Felt::new(64)),
            ),
            (
                StoragePlaceholder::new("word.test").unwrap(),
                StorageValue::Word([Felt::ZERO, Felt::ZERO, Felt::ZERO, Felt::new(128)]),
            ),
            (
                StoragePlaceholder::new("map.template").unwrap(),
                StorageValue::Map(StorageMap::default()),
            ),
        ]);

        let component = AccountComponent::from_template(&template, &storage_placeholders).unwrap();

        let storage_map = component.storage_slots.first().unwrap();
        match storage_map {
            StorageSlot::Map(storage_map) => assert_eq!(storage_map.entries().count(), 3),
            _ => panic!("should be map"),
        }

        let value_entry = component.storage_slots().get(1).unwrap();
        match value_entry {
            StorageSlot::Value(v) => {
                assert_eq!(v, &[Felt::ZERO, Felt::ZERO, Felt::ZERO, Felt::new(128)])
            },
            _ => panic!("should be value"),
        }

        let failed_instantiation =
            AccountComponent::from_template(&template, &InitStorageData::default());
        assert_matches!(
            failed_instantiation,
            Err(AccountError::AccountComponentTemplateInstantiationError(
                AccountComponentTemplateError::PlaceholderValueNotProvided(_)
            ))
        );
    }

    #[test]
    pub fn fail_placeholder_type_mismatch() {
        let toml_text = r#"
            name = "Test Component"
            description = "This is a test component"
            version = "1.0.1"
            targets = ["FungibleFaucet"]

            [[storage]]
            name = "map"
            description = "A storage map entry"
            slot = 0
            values = [
                { key = "0x1", value = ["{{value.test}}", "0x1", "0x2", "0x3"] },
            ]

            [[storage]]
            name = "word"
            slot = 1
            value = "{{value.test}}"
        "#;
        let component_metadata = AccountComponentMetadata::from_toml(toml_text);
        assert_matches!(
            component_metadata,
            Err(AccountComponentTemplateError::StoragePlaceholderTypeMismatch(_, _, _))
        );
    }
}
