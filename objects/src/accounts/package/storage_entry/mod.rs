use alloc::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use vm_core::Word;
use vm_processor::Digest;

mod word;
pub use word::*;

use super::AccountComponentTemplateError;
use crate::accounts::{StorageMap, StorageSlot};

mod template;
pub use template::{TemplateKey, TemplateValue};

// STORAGE ENTRY
// ================================================================================================

/// Represents a single entry in the component’s storage layout.
///
/// Each entry can describe:
/// - A value slot (single word or multiple words).
/// - A map slot (key-value map that occupies one storage slot).
/// - A multi-slot entry (spanning multiple contiguous slots, with multiple values).
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
        values: Vec<MapEntry>,
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
        storage_map: StorageMap,
    ) -> Self {
        let map_entries = storage_map
            .entries()
            .copied()
            .map(|(k, v)| MapEntry::new(Into::<Word>::into(k), v))
            .collect();
        StorageEntry::Map {
            name: name.into(),
            description: description.map(Into::<String>::into),
            slot,
            values: map_entries,
        }
    }

    /// Creates a new [`StorageEntry::MultiSlot`] variant.
    pub fn new_multi_slot(
        name: impl Into<String>,
        description: Option<impl Into<String>>,
        slots: Vec<u8>,
        values: Vec<impl Into<WordRepresentation>>,
    ) -> Result<Self, AccountComponentTemplateError> {
        if slots.len() != values.len() {
            return Err(AccountComponentTemplateError::InvalidMultiSlotEntry);
        }

        for window in slots.windows(2) {
            if window[1] != window[0] + 1 {
                return Err(AccountComponentTemplateError::NonContiguousSlots);
            }
        }

        Ok(StorageEntry::MultiSlot {
            name: name.into(),
            description: description.map(Into::<String>::into),
            slots,
            values: values.into_iter().map(Into::into).collect(),
        })
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
            StorageEntry::Value { value, .. } => std::slice::from_ref(value),
            StorageEntry::MultiSlot { values, .. } => values.as_slice(),
            StorageEntry::Map { .. } => &[],
        }
    }

    /// Returns the map entries for a `Map` variant as a slice.
    /// Returns an empty slice for non-map variants.
    pub fn map_entries(&self) -> &[MapEntry] {
        match self {
            StorageEntry::Map { values, .. } => values.as_slice(),
            StorageEntry::Value { .. } => &[],
            StorageEntry::MultiSlot { .. } => &[],
        }
    }

    /// Returns an iterator over all of the storage entries's template keys.
    // TODO: Should template keys be typed?
    pub fn template_keys(&self) -> Box<dyn Iterator<Item = &TemplateKey> + '_> {
        match self {
            StorageEntry::Value { value, .. } => value.template_keys(),
            StorageEntry::Map { values, .. } => {
                Box::new(values.iter().flat_map(|word| word.template_keys()))
            },
            StorageEntry::MultiSlot { values, .. } => {
                Box::new(values.iter().flat_map(|word| word.template_keys()))
            },
        }
    }

    /// Attempts to convert the storage entry into a list of [StorageSlot].
    ///
    /// - StorageEntry::Value would convert to a [StorageSlot::Value]
    /// - StorageEntry::MultiSlot would convert to as many [StorageSlot::Value] as defined
    /// - StorageEntry::Map would convert to a [StorageSlot::Map]
    ///
    /// Each of the entry's values could be dynamic. These values are replaced for values found
    /// in `template_values`, identified by its key.
    pub fn try_into_storage_slots(
        self,
        template_values: &BTreeMap<String, TemplateValue>,
    ) -> Result<Vec<StorageSlot>, AccountComponentTemplateError> {
        match self {
            StorageEntry::Value { value, .. } => {
                let slot = value.try_into_word(template_values)?;
                Ok(vec![StorageSlot::Value(slot)])
            },
            StorageEntry::Map { values, .. } => {
                let entries = values
                    .into_iter()
                    .map(|map_entry| {
                        let (key, value) = map_entry.into_parts();
                        let key = key.try_into_word(template_values)?;
                        let value = value.try_into_word(template_values)?;
                        Ok((key.into(), value))
                    })
                    .collect::<Result<Vec<(Digest, Word)>, AccountComponentTemplateError>>()?; // Collect into a Vec and propagate errors

                let storage_map = StorageMap::with_entries(entries)
                    .map_err(AccountComponentTemplateError::StorageMapError)?;
                Ok(vec![StorageSlot::Map(storage_map)])
            },
            StorageEntry::MultiSlot { values, .. } => Ok(values
                .into_iter()
                .map(|word_repr| word_repr.try_into_word(template_values).map(StorageSlot::Value))
                .collect::<Result<Vec<StorageSlot>, _>>()?),
        }
    }
}

// SERIALIZATION
// ================================================================================================

#[derive(Default, Serialize, Deserialize)]
/// Used as a helper for validating and (de)serializing storage entries
struct RawStorageEntry {
    name: String,
    description: Option<String>,
    slot: Option<u8>,
    slots: Option<Vec<u8>>,
    value: Option<WordRepresentation>,
    values: Option<StorageValues>,
}

impl From<StorageEntry> for RawStorageEntry {
    fn from(entry: StorageEntry) -> Self {
        match entry {
            StorageEntry::Value { name, description, slot, value } => RawStorageEntry {
                name,
                description,
                slot: Some(slot),
                value: Some(value),
                ..Default::default()
            },
            StorageEntry::Map { name, description, slot, values } => RawStorageEntry {
                name,
                description,
                slot: Some(slot),
                values: Some(StorageValues::MapEntries(values)),
                ..Default::default()
            },
            StorageEntry::MultiSlot { name, description, slots, values } => RawStorageEntry {
                name,
                description,
                slots: Some(slots),
                values: Some(StorageValues::Words(values)),
                ..Default::default()
            },
        }
    }
}

impl Serialize for StorageEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let raw_storage_entry: RawStorageEntry = self.clone().into();
        raw_storage_entry.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StorageEntry {
    fn deserialize<D>(deserializer: D) -> Result<StorageEntry, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawStorageEntry::deserialize(deserializer)?;

        // Determine presence of fields and do early validation
        let slot_present = raw.slot.is_some();
        let slots_present = raw.slots.is_some();
        let value_present = raw.value.is_some();
        let values_present = raw.values.is_some();

        // Use a match on the combination of presence flags to choose variant
        match (raw.slots, raw.values) {
            (None, None) => {
                // Expect a Value variant: "slot" and "value" must be present
                // "slots" and "values" must not be present.
                Ok(StorageEntry::Value {
                    name: raw.name,
                    description: raw.description,
                    slot: raw
                        .slot
                        .ok_or(D::Error::custom("Missing 'slot' field for single-slot entry."))?,
                    value: raw
                        .value
                        .ok_or(D::Error::custom("Missing 'value' field for single-slot entry."))?,
                })
            },
            (Some(_), None) => {
                Err(D::Error::custom("`slots` is defined but no `values` field was found."))
            },
            (None, Some(values)) => {
                // Expect a Map variant:
                //   - `slot` must be present
                //   - `values` must be present and convertible to map entries
                //   - `slots` must not be present
                //   - `value` must not be present
                if value_present {
                    return Err(D::Error::custom(
                        "Fields 'value' and 'values' are mutually exclusive",
                    ));
                }

                let map_entries = values
                    .into_map_entries()
                    .ok_or_else(|| D::Error::custom("Invalid 'values' for map entry"))?;

                Ok(StorageEntry::Map {
                    name: raw.name,
                    description: raw.description,
                    slot: raw.slot.ok_or(D::Error::missing_field("slot"))?,
                    values: map_entries,
                })
            },
            (Some(slots), Some(values)) => {
                // Expect a MultiSlot variant:
                //   - `slots` must be present
                //   - `values` must be present and represent words
                //   - `slot` must not be present
                //   - `value` must not be present
                if slot_present {
                    return Err(D::Error::custom(
                        "Fields 'slot' and 'slots' are mutually exclusive.",
                    ));
                }
                if value_present {
                    return Err(D::Error::custom(
                        "Fields 'value' and 'values' are mutually exclusive.",
                    ));
                }
                
                let has_list_of_values = values.is_list_of_words();
                if has_list_of_values {
                    let slots_count = slots.len();
                    let values_count = values.len().expect("checked that it's a list of values");
                    if slots_count != values_count {
                        return Err(D::Error::custom(format!(
                            "Number of slots ({}) does not match number of values ({}) for multi-slot storage entry.",
                            slots_count, values_count
                        )));
                    }
                }

                Ok(StorageEntry::MultiSlot {
                    name: raw.name,
                    description: raw.description,
                    slots,
                    values: values
                        .into_words()
                        .ok_or_else(|| D::Error::custom("Invalid values for multi-slot."))?,
                })
            },
        }
    }
}

// STORAGE VALUES
// ================================================================================================

/// Represents the type of values that can be found in a storage slot's `values` field.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum StorageValues {
    /// List of individual words (for multi-slot entries).
    Words(Vec<WordRepresentation>),
    /// List of key-value entries (for map storage slots).
    MapEntries(Vec<MapEntry>),
    /// A template written as "{{key}}".
    Dynamic(TemplateKey),
}

impl StorageValues {
    pub fn is_list_of_words(&self) -> bool {
        match self {
            StorageValues::Words(_) => true,
            StorageValues::MapEntries(_) => false,
            StorageValues::Dynamic(_) => false,
        }
    }

    pub fn into_words(self) -> Option<Vec<WordRepresentation>> {
        match self {
            StorageValues::Words(vec) => Some(vec),
            StorageValues::MapEntries(_) => None,
            StorageValues::Dynamic(_) => None,
        }
    }

    pub fn into_map_entries(self) -> Option<Vec<MapEntry>> {
        match self {
            StorageValues::Words(_) => None,
            StorageValues::MapEntries(vec) => Some(vec),
            StorageValues::Dynamic(_) => None,
        }
    }

    pub fn len(&self) -> Option<usize> {
        match self {
            StorageValues::Words(vec) => Some(vec.len()),
            StorageValues::MapEntries(vec) => Some(vec.len()),
            StorageValues::Dynamic(_) => None,
        }
    }
}

// MAP ENTRY
// ================================================================================================

/// Key-value entry for storage maps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapEntry {
    key: WordRepresentation,
    value: WordRepresentation,
}

impl MapEntry {
    pub fn new(key: impl Into<WordRepresentation>, value: impl Into<WordRepresentation>) -> Self {
        Self { key: key.into(), value: value.into() }
    }

    pub fn template_keys(&self) -> impl Iterator<Item = &TemplateKey> {
        self.key.template_keys().chain(self.value.template_keys())
    }

    pub fn into_parts(self) -> (WordRepresentation, WordRepresentation) {
        let MapEntry { key, value } = self;
        (key, value)
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, string::ToString};

    use assembly::Assembler;
    use assert_matches::assert_matches;
    use semver::Version;
    use toml;
    use vm_core::{Felt, FieldElement};

    use super::*;
    use crate::{
        accounts::{
            package::{ComponentMetadata, AccountComponentTemplate},
            AccountComponent, AccountType,
        },
        digest,
        testing::account_code::CODE,
    };

    #[test]
    fn test_storage_entry_serialization() {
        let array = [
            FeltRepresentation::SingleDecimal(Felt::new(0xabc)),
            FeltRepresentation::SingleDecimal(Felt::new(1218)),
            FeltRepresentation::SingleHex(Felt::new(0xdba3)),
            FeltRepresentation::Dynamic(TemplateKey::new("test.array.dyn")),
        ];
        let storage = vec![
            StorageEntry::Value {
                name: "slot0".into(),
                description: Some("First slot".into()),
                slot: 0,
                value: WordRepresentation::SingleHex(digest!("0x333123").into()),
            },
            StorageEntry::Map {
                name: "map".into(),
                description: Some("A storage map entry".into()),
                slot: 1,
                values: vec![
                    MapEntry {
                        key: WordRepresentation::Dynamic(TemplateKey::new("foo.bar")),
                        value: WordRepresentation::SingleHex(digest!("0x2").into()),
                    },
                    MapEntry {
                        key: WordRepresentation::SingleHex(digest!("0x2").into()),
                        value: WordRepresentation::Dynamic(TemplateKey::new("bar.baz")),
                    },
                    MapEntry {
                        key: WordRepresentation::SingleHex(digest!("0x3").into()),
                        value: WordRepresentation::SingleHex(digest!("0x4").into()),
                    },
                ],
            },
            StorageEntry::MultiSlot {
                name: "multi".into(),
                description: Some("Multi slot entry".into()),
                slots: vec![2, 3, 4],
                values: vec![
                    WordRepresentation::Dynamic(TemplateKey::new("test.dynamic")),
                    WordRepresentation::Array(array),
                    WordRepresentation::SingleHex(digest!("0xabcdef123abcdef123").into()),
                ],
            },
            StorageEntry::Value {
                name: "single-slot".into(),
                description: Some("Slot with dynamic key".into()),
                slot: 0,
                value: WordRepresentation::Dynamic(TemplateKey::new("single-slot-key")),
            },
        ];

        let config = ComponentMetadata {
            name: "Test Component".into(),
            description: "This is a test component".into(),
            version: Version::parse("1.0.0").unwrap(),
            targets: BTreeSet::from([AccountType::FungibleFaucet]),
            storage,
        };

        let toml = toml::to_string(&config).unwrap();

        let deserialized: ComponentMetadata =
            toml::from_str(&toml).expect("Deserialization failed");
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
                { key = "{{key.test}}", value = "0x3" },
                { key = "0x3", value = "0x4" }
            ]

            [[storage]]
            name = "test-word"
            description = "word"
            slot = 1
            value = "{{word.test}}" 
        "#;

        let component_metadata = ComponentMetadata::from_toml(toml_text).unwrap();
        let library = Assembler::default().assemble_library([CODE]).unwrap();

        assert_eq!(component_metadata.storage_entries().first().unwrap().map_entries().len(), 3);

        let package = AccountComponentTemplate::new(component_metadata, library);
        let template_keys = [
            ("key.test".to_string(), TemplateValue::Word(Default::default())),
            ("value.test".to_string(), TemplateValue::Felt(Felt::new(64))),
            (
                "word.test".to_string(),
                TemplateValue::Word([Felt::ZERO, Felt::ZERO, Felt::ZERO, Felt::new(128)]),
            ),
        ]
        .into_iter()
        .collect();

        let component = AccountComponent::from_template(&package, &template_keys).unwrap();
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

        let failed_instantiation = AccountComponent::from_template(&package, &BTreeMap::new());
        assert_matches!(
            failed_instantiation,
            Err(AccountComponentTemplateError::TemplateValueNotProvided(_))
        );
    }
}
