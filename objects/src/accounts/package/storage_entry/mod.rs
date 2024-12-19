use alloc::{string::String, vec::Vec};

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

mod word;
pub use word::*;

// STORAGE ENTRY
// ================================================================================================

/// Represents a single entry in the component’s storage layout.
///
/// Each entry can describe:
/// - A value slot (single word or multiple words).
/// - A map slot (key-value map that occupies one storage slot).
/// - A multi-slot entry (spanning multiple contiguous slots, with multipe values).
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
    /// Returns the slot indices that the storage entry covers.
    pub fn slot_indices(&self) -> &[u8] {
        match self {
            StorageEntry::MultiSlot { slots, .. } => slots.as_slice(),
            StorageEntry::Value { slot, .. } => std::slice::from_ref(slot),
            StorageEntry::Map { slot, .. } => std::slice::from_ref(slot),
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

        // Determine presence of fields
        let slot_present = raw.slot.is_some();
        let slots_present = raw.slots.is_some();
        let value_present = raw.value.is_some();
        let values_present = raw.values.is_some();

        // Use a match on the combination of presence flags to choose variant
        match (slots_present, values_present) {
            (false, false) => {
                // Expect a Value variant: "slot" and "value" must be present
                // "slots" and "values" must not be present.
                if !slot_present {
                    return Err(D::Error::custom("Missing 'slot' field for single-slot entry."));
                }
                if !value_present {
                    return Err(D::Error::custom("Missing 'value' field for single-slot entry."));
                }
                Ok(StorageEntry::Value {
                    name: raw.name,
                    description: raw.description,
                    slot: raw.slot.expect("was checked to be present"),
                    value: raw.value.expect("was checked to be present"),
                })
            },
            (true, false) => {
                Err(D::Error::custom("`slots` is defined but no `values` field was found."))
            },
            (false, true) => {
                // Expect a Map variant:
                //   - `slot` must be present
                //   - `values` must be present and convertible to map entries
                //   - `slots` must not be present
                //   - `value` must not be present
                if !slot_present {
                    return Err(D::Error::missing_field("slot"));
                }
                if raw.value.is_some() {
                    return Err(D::Error::custom(
                        "Fields 'value' and 'values' are mutually exclusive",
                    ));
                }

                let values = raw.values.expect("was checked to be present");
                let map_entries = values
                    .into_map_entries()
                    .ok_or_else(|| D::Error::custom("Invalid 'values' for map entry"))?;

                Ok(StorageEntry::Map {
                    name: raw.name,
                    description: raw.description,
                    slot: raw.slot.expect("was checked to be present"),
                    values: map_entries,
                })
            },
            (true, true) => {
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
                let slots = raw.slots.ok_or_else(|| D::Error::missing_field("slots"))?;
                let values = raw.values.ok_or_else(|| D::Error::missing_field("values"))?;

                let has_list_of_values = values.is_list_of_words();
                if has_list_of_values {
                    let slots_count = slots.len();
                    let values_count = values.len();
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
}

impl StorageValues {
    pub fn is_list_of_words(&self) -> bool {
        match self {
            StorageValues::Words(_) => true,
            StorageValues::MapEntries(_) => false,
        }
    }

    pub fn into_words(self) -> Option<Vec<WordRepresentation>> {
        match self {
            StorageValues::Words(vec) => Some(vec),
            StorageValues::MapEntries(_) => None,
        }
    }

    pub fn into_map_entries(self) -> Option<Vec<MapEntry>> {
        match self {
            StorageValues::Words(_) => None,
            StorageValues::MapEntries(vec) => Some(vec),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            StorageValues::Words(vec) => vec.len(),
            StorageValues::MapEntries(vec) => vec.len(),
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use semver::Version;
    use toml;
    use vm_core::Felt;

    use super::*;
    use crate::{
        accounts::{package::ComponentMetadata, AccountType},
        digest,
    };

    #[test]
    fn test_storage_entry_serialization() {
        let array = [
            FeltRepresentation::SingleDecimal(Felt::new(91)),
            FeltRepresentation::SingleDecimal(Felt::new(1218)),
            FeltRepresentation::SingleHex(Felt::new(0xdba3)),
            FeltRepresentation::SingleHex(Felt::new(0xfffeeff)),
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
                        key: WordRepresentation::SingleHex(digest!("0x1").into()),
                        value: WordRepresentation::SingleHex(digest!("0x2").into()),
                    },
                    MapEntry {
                        key: WordRepresentation::SingleHex(digest!("0x2").into()),
                        value: WordRepresentation::SingleHex(digest!("0x3").into()),
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
                slots: vec![2, 3],
                values: vec![
                    WordRepresentation::Array(array),
                    WordRepresentation::SingleHex(digest!("0xabcdef123abcdef123").into()),
                ],
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
            slot = 1
            values = [
                { key = "0x1", value = "0x2" },
                { key = "0x2", value = "0x3" },
                { key = "0x3", value = "0x4" }
            ]
        "#;

        let component_metadata: ComponentMetadata = toml::from_str(toml_text).unwrap();
        assert_eq!(component_metadata.storage().first().unwrap().map_entries().len(), 3)
    }
}
