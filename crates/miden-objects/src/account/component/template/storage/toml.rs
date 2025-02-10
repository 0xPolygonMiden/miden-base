use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use serde::{
    de::{value::MapAccessDeserializer, Error, SeqAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};

use super::{FeltRepresentation, MapEntry, MapRepresentation, StorageEntry, WordRepresentation};
use crate::{
    account::{
        component::template::storage::placeholder::{parse_felt_from_str, FeltType, WordType},
        AccountComponentMetadata, StorageValueName,
    },
    errors::AccountComponentTemplateError,
    utils::parse_hex_string_as_word,
};

// ACCOUNT COMPONENT METADATA TOML FROM/TO
// ================================================================================================

impl AccountComponentMetadata {
    /// Deserializes `toml_string` and validates the resulting [AccountComponentMetadata]
    ///
    /// # Errors
    ///
    /// - If deserialization fails
    /// - If the template specifies storage slots with duplicates.
    /// - If the template includes slot numbers that do not start at zero.
    /// - If storage slots in the template are not contiguous.
    pub fn from_toml(toml_string: &str) -> Result<Self, AccountComponentTemplateError> {
        let component: AccountComponentMetadata = toml::from_str(toml_string)
            .map_err(AccountComponentTemplateError::DeserializationError)?;

        component.validate()?;
        Ok(component)
    }

    /// Serializes the account component template into a TOML string.
    pub fn as_toml(&self) -> Result<String, AccountComponentTemplateError> {
        let toml = toml::to_string_pretty(self)
            .map_err(AccountComponentTemplateError::SerializationError)?;
        Ok(toml)
    }
}

// WORD REPRESENTATION SERIALIZATION
// ================================================================================================

impl Serialize for WordRepresentation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            WordRepresentation::Template { name, description, r#type } => {
                // Serialize as a table with keys: "name", "description", "type".
                let mut state = serializer.serialize_struct("WordRepresentation", 3)?;
                state.serialize_field("name", name)?;
                state.serialize_field("description", description)?;
                state.serialize_field("type", r#type)?;
                state.end()
            },
            WordRepresentation::Value { name, description, value } => {
                // Serialize as a table with keys: "name", "description", "value".
                let mut state = serializer.serialize_struct("WordRepresentation", 3)?;
                state.serialize_field("name", name)?;
                state.serialize_field("description", description)?;
                state.serialize_field("value", value)?;
                state.end()
            },
        }
    }
}

impl<'de> Deserialize<'de> for WordRepresentation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct WordRepresentationVisitor;

        impl<'de> Visitor<'de> for WordRepresentationVisitor {
            type Value = WordRepresentation;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or a map representing a WordRepresentation")
            }

            // A bare stirng is interpreted it as a Value variant.
            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                let parsed_value = parse_hex_string_as_word(value).map_err(|_err| {
                    E::invalid_value(
                        serde::de::Unexpected::Str(value),
                        &"a valid hexadecimal string",
                    )
                })?;
                Ok(parsed_value.into())
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: Error,
            {
                self.visit_str(&value)
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                // Deserialize as a list of felt representations
                let elements: Vec<FeltRepresentation> =
                    Deserialize::deserialize(serde::de::value::SeqAccessDeserializer::new(seq))?;
                if elements.len() != 4 {
                    return Err(Error::invalid_length(
                        elements.len(),
                        &"expected an array of 4 elements",
                    ));
                }
                let array: [FeltRepresentation; 4] =
                    elements.try_into().expect("length was checked");
                Ok(WordRepresentation::new_value(array, None, None))
            }

            fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                #[derive(Deserialize, Debug)]
                struct WordRepresentationHelper {
                    name: Option<String>,
                    description: Option<String>,
                    // The "value" field (if present) must be an array of 4 FeltRepresentations.
                    value: Option<[FeltRepresentation; 4]>,
                    #[serde(rename = "type")]
                    r#type: Option<WordType>,
                }

                let helper =
                    WordRepresentationHelper::deserialize(MapAccessDeserializer::new(map))?;

                // If a value field is present, assume a Value variant.
                if let Some(val) = helper.value {
                    let name = helper.name.map(parse_name).transpose()?;
                    Ok(WordRepresentation::new_value(val, name, helper.description))
                } else {
                    // Otherwise, we expect a Template variant (name is required for identification)
                    let name = expect_parse_value_name(helper.name, "word template")?;

                    // If type not defined, assume Word
                    let r#type = helper.r#type.unwrap_or(WordType::Words(1));
                    Ok(WordRepresentation::new_template(name, helper.description, r#type))
                }
            }
        }

        deserializer.deserialize_any(WordRepresentationVisitor)
    }
}

// FELT REPRESENTATION SERIALIZATION
// ================================================================================================

impl Serialize for FeltRepresentation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            FeltRepresentation::Value { name, description, value } => {
                let hex = value.to_string();
                if name.is_none() && description.is_none() {
                    serializer.serialize_str(&hex)
                } else {
                    let mut state = serializer.serialize_struct("FeltRepresentation", 3)?;
                    state.serialize_field("name", name)?;
                    state.serialize_field("description", description)?;
                    state.serialize_field("value", &hex)?;
                    state.end()
                }
            },
            FeltRepresentation::Template { name, description, r#type } => {
                let mut state = serializer.serialize_struct("FeltRepresentation", 3)?;
                state.serialize_field("name", name)?;
                state.serialize_field("description", description)?;
                state.serialize_field("type", r#type)?;
                state.end()
            },
        }
    }
}

impl<'de> Deserialize<'de> for FeltRepresentation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Felts can be deserialized as either:
        //
        // - Scalars (parsed from strings)
        // - A table object that can or cannot harcode a value. If not present, this is a
        //   placeholder type
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Intermediate {
            Map {
                name: Option<String>,
                description: Option<String>,
                #[serde(default)]
                value: Option<String>,
                #[serde(rename = "type")]
                r#type: Option<FeltType>,
            },
            Scalar(String),
        }

        let intermediate = Intermediate::deserialize(deserializer)?;
        match intermediate {
            Intermediate::Scalar(s) => {
                let felt = parse_felt_from_str(&s)
                    .map_err(|e| D::Error::custom(format!("failed to parse Felt: {}", e)))?;
                Ok(FeltRepresentation::Value {
                    name: None,
                    description: None,
                    value: felt,
                })
            },
            Intermediate::Map { name, description, value, r#type } => {
                // Get the defined type, or fall back to FeltType::Felt as default
                let felt_type = r#type.unwrap_or_default();

                if let Some(val_str) = value {
                    // Parse into felt from the input string
                    let felt = felt_type
                        .parse_value(&val_str)
                        .map_err(|e| D::Error::custom(format!("failed to parse Felt: {}", e)))?;
                    let name = name.map(parse_name).transpose()?;
                    Ok(FeltRepresentation::new_value(felt, name, description))
                } else {
                    // No value provided, so this is a placeholder
                    let name = expect_parse_value_name(name, "map template")?;

                    Ok(FeltRepresentation::new_template(felt_type, name, description))
                }
            },
        }
    }
}

// STORAGE VALUES
// ================================================================================================

/// Represents the type of values that can be found in a storage slot's `values` field.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StorageValues {
    /// List of individual words (for multi-slot entries).
    Words(Vec<[FeltRepresentation; 4]>),
    /// List of key-value entries (for map storage slots).
    MapEntries(Vec<MapEntry>),
}

// NOTE: We serialize manually here for forcing inline collection
impl Serialize for StorageValues {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            StorageValues::Words(arrays) => serializer.collect_seq(arrays),
            StorageValues::MapEntries(entries) => serializer.collect_seq(entries),
        }
    }
}

// STORAGE ENTRY SERIALIZATION
// ================================================================================================

#[derive(Default, Debug, Deserialize, Serialize)]
struct RawStorageEntry {
    name: Option<String>,
    description: Option<String>,
    slot: Option<u8>,
    slots: Option<Vec<u8>>,
    #[serde(rename = "type")]
    word_type: Option<WordType>,
    value: Option<[FeltRepresentation; 4]>,
    values: Option<StorageValues>,
}

impl From<StorageEntry> for RawStorageEntry {
    fn from(entry: StorageEntry) -> Self {
        match entry {
            StorageEntry::Value { slot, word_entry } => match word_entry {
                WordRepresentation::Value { name, description, value } => RawStorageEntry {
                    slot: Some(slot),
                    name: name.as_ref().map(StorageValueName::to_string),
                    description: description.map(String::from),
                    value: Some(value),
                    ..Default::default()
                },
                WordRepresentation::Template { name, description, r#type } => RawStorageEntry {
                    slot: Some(slot),
                    name: Some(name.to_string()),
                    description: description.map(String::from),
                    word_type: Some(r#type),
                    ..Default::default()
                },
            },
            StorageEntry::Map { slot, map } => RawStorageEntry {
                name: Some(map.name().to_string()),
                description: map.description().cloned(),
                slot: Some(slot),
                values: Some(StorageValues::MapEntries(map.entries().to_vec())),
                ..Default::default()
            },
            StorageEntry::MultiSlot { name, description, slots, values } => RawStorageEntry {
                name: Some(name.to_string()),
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

        if let Some(word_entry) = raw.value {
            // If a value was provided, this is a WordRepresentation::Value entry
            let slot = raw.slot.ok_or_else(|| missing_field_for("slot", "value entry"))?;
            let name = raw.name.map(parse_name).transpose()?;

            Ok(StorageEntry::Value {
                slot,
                word_entry: WordRepresentation::new_value(word_entry, name, raw.description),
            })
        } else if let Some(word_type) = raw.word_type {
            // If a type was provided instead, this is a WordRepresentation::Value entry
            let slot = raw.slot.ok_or_else(|| missing_field_for("slot", "single-slot entry"))?;
            let name = expect_parse_value_name(raw.name, "single-slot entry")?;
            let word_entry = WordRepresentation::new_template(name, raw.description, word_type);

            Ok(StorageEntry::Value { slot, word_entry })
        } else if let Some(StorageValues::MapEntries(map_entries)) = raw.values {
            // If `values` field contains key/value pairs, deserialize as map
            let name = expect_parse_value_name(raw.name, "map entry")?;
            let slot = raw.slot.ok_or_else(|| missing_field_for("slot", "map entry"))?;
            let map = MapRepresentation::new(map_entries, name, raw.description);

            Ok(StorageEntry::Map { slot, map })
        } else if let Some(StorageValues::Words(values)) = raw.values {
            let name = expect_parse_value_name(raw.name, "multislot entry")?;
            let slots = raw.slots.ok_or_else(|| missing_field_for("slots", "multislot entry"))?;

            if slots.len() != values.len() {
                return Err(D::Error::custom(format!(
                    "number of slots ({}) does not match number of values ({}) for multislot entry",
                    slots.len(),
                    values.len()
                )));
            }
            Ok(StorageEntry::new_multislot(name, raw.description, slots, values))
        } else {
            Err(D::Error::custom("invalid combination of fields for storage entry"))
        }
    }
}

// UTILS / HELPERS
// ================================================================================================

fn missing_field_for<E: serde::de::Error>(field: &str, context: &str) -> E {
    E::custom(format!("missing '{}' field for {}", field, context))
}

/// Checks than an optional (but expected) name field has been defined and is correct.
fn expect_parse_value_name<E: serde::de::Error>(
    n: Option<String>,
    context: &str,
) -> Result<StorageValueName, E> {
    let name = n.ok_or_else(|| missing_field_for("name", context))?;
    parse_name(name)
}

/// Tries to parse a string into a [StorageValueName].
fn parse_name<E: serde::de::Error>(n: String) -> Result<StorageValueName, E> {
    StorageValueName::new(n).map_err(|err| E::custom(format!("invalid `name`: {err}")))
}
