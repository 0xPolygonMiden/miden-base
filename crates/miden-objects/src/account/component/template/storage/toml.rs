use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;
use std::collections::BTreeMap;

use serde::{
    de::{value::MapAccessDeserializer, Error, SeqAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use thiserror::Error;
use vm_core::Felt;

use super::{
    placeholder::TemplateType, FeltRepresentation, InitStorageData, MapEntry, MapRepresentation,
    StorageEntry, StorageValueNameError, WordRepresentation,
};
use crate::{
    account::{
        component::template::storage::placeholder::{TemplateFelt, TEMPLATE_REGISTRY},
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
            .map_err(AccountComponentTemplateError::TomlDeserializationError)?;

        component.validate()?;
        Ok(component)
    }

    /// Serializes the account component template into a TOML string.
    pub fn as_toml(&self) -> Result<String, AccountComponentTemplateError> {
        let toml = toml::to_string_pretty(self)
            .map_err(AccountComponentTemplateError::TomlSerializationError)?;
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

            // A bare string is interpreted it as a Value variant.
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
                    r#type: Option<TemplateType>,
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

                    // Get the type, or the default if it was not specified
                    let r#type = helper.r#type.unwrap_or(TemplateType::default_word_type());
                    Ok(WordRepresentation::new_template(r#type, name, helper.description))
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
                r#type: Option<TemplateType>,
            },
            Scalar(String),
        }

        let intermediate = Intermediate::deserialize(deserializer)?;
        match intermediate {
            Intermediate::Scalar(s) => {
                let felt = Felt::parse_felt(&s)
                    .map_err(|e| D::Error::custom(format!("failed to parse Felt: {}", e)))?;
                Ok(FeltRepresentation::Value {
                    name: None,
                    description: None,
                    value: felt,
                })
            },
            Intermediate::Map { name, description, value, r#type } => {
                // Get the defined type, or the default if it was not specified
                let felt_type = r#type.unwrap_or(TemplateType::default_felt_type());

                if let Some(val_str) = value {
                    // Parse into felt from the input string
                    let felt =
                        TEMPLATE_REGISTRY.try_parse_felt(&felt_type, &val_str).map_err(|e| {
                            D::Error::custom(format!("failed to parse {felt_type} as Felt: {}", e))
                        })?;
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
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum StorageValues {
    /// List of individual words (for multi-slot entries).
    Words(Vec<[FeltRepresentation; 4]>),
    /// List of key-value entries (for map storage slots).
    MapEntries(Vec<MapEntry>),
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
    word_type: Option<TemplateType>,
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
                values: Some(StorageValues::MapEntries(map.into())),
                ..Default::default()
            },
            StorageEntry::MultiSlot { name, description, slots, values } => RawStorageEntry {
                name: Some(name.to_string()),
                description,
                slots: Some(slots.collect()),
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
        } else if let Some(StorageValues::MapEntries(map_entries)) = raw.values {
            // If `values` field contains key/value pairs, deserialize as map
            let name = expect_parse_value_name(raw.name, "map entry")?;
            let slot = raw.slot.ok_or_else(|| missing_field_for("slot", "map entry"))?;
            let map = MapRepresentation::new(map_entries, name, raw.description);

            Ok(StorageEntry::Map { slot, map })
        } else if let Some(StorageValues::Words(values)) = raw.values {
            let name = expect_parse_value_name(raw.name, "multislot entry")?;
            let mut slots =
                raw.slots.ok_or_else(|| missing_field_for("slots", "multislot entry"))?;

            // Sort so we can check contiguity
            slots.sort_unstable();

            for pair in slots.windows(2) {
                if pair[1] != pair[0] + 1 {
                    return Err(serde::de::Error::custom(format!(
                        "`slots` field in `{name}` entry is not a valid range"
                    )));
                }
            }

            let start = slots[0];
            let end = slots.last().expect("checked validity") + 1;

            Ok(StorageEntry::new_multislot(name, raw.description, start..end, values))
        } else if let Some(word_type) = raw.word_type {
            // If a type was provided instead, this is a WordRepresentation::Value entry
            let slot = raw.slot.ok_or_else(|| missing_field_for("slot", "single-slot entry"))?;
            let name = expect_parse_value_name(raw.name, "single-slot entry")?;
            let word_entry = WordRepresentation::new_template(word_type, name, raw.description);

            Ok(StorageEntry::Value { slot, word_entry })
        } else {
            Err(D::Error::custom("placeholder storage entries require the `type` field"))
        }
    }
}

// INIT STORAGE DATA
// ================================================================================================

impl InitStorageData {
    /// Creates an instance of [`InitStorageData`] from a TOML string.
    ///
    /// This method parses the provided TOML and flattens nested tables into
    /// dot‑separated keys using [`StorageValueName`] as keys. All values are converted to plain
    /// strings (so that, for example, `key = 10` and `key = "10"` both yield
    /// `String::from("10")` as the value).
    ///
    /// # Errors
    ///
    /// - If duplicate keys or empty tables are found in the string
    /// - If the TOML string includes arrays
    pub fn from_toml(toml_str: &str) -> Result<Self, InitStorageDataError> {
        let value: toml::Value = toml::from_str(toml_str)?;
        let mut placeholders = BTreeMap::new();
        // Start with an empty prefix (i.e. the default, which is an empty string)
        Self::flatten_parse_toml_value(StorageValueName::default(), &value, &mut placeholders)?;
        Ok(InitStorageData::new(placeholders))
    }

    /// Recursively flattens a TOML `Value` into a flat mapping.
    ///
    /// When recursing into nested tables, keys are combined using
    /// [`StorageValueName::with_suffix`]. If an encountered table is empty (and not the top-level),
    /// an error is returned. Arrays are not supported.
    fn flatten_parse_toml_value(
        prefix: StorageValueName,
        value: &toml::Value,
        map: &mut BTreeMap<StorageValueName, String>,
    ) -> Result<(), InitStorageDataError> {
        match value {
            toml::Value::Table(table) => {
                // If this is not the root and the table is empty, error
                if !prefix.as_str().is_empty() && table.is_empty() {
                    return Err(InitStorageDataError::EmptyTable(prefix.as_str().into()));
                }
                for (key, val) in table {
                    // Create a new key and combine it with the current prefix.
                    let new_key = StorageValueName::new(key.to_string())
                        .map_err(InitStorageDataError::InvalidStorageValueName)?;
                    let new_prefix = prefix.clone().with_suffix(&new_key);
                    Self::flatten_parse_toml_value(new_prefix, val, map)?;
                }
            },
            toml::Value::Array(_) => {
                return Err(InitStorageDataError::ArraysNotSupported);
            },
            toml_value => {
                // Get the string value, or convert to string if it's some other type
                let value = match toml_value {
                    toml::Value::String(s) => s.clone(),
                    _ => value.to_string(),
                };
                map.insert(prefix, value);
            },
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum InitStorageDataError {
    #[error("failed to parse TOML: {0}")]
    InvalidToml(#[from] toml::de::Error),

    #[error("empty table encountered for key `{0}`")]
    EmptyTable(String),

    #[error("invalid input: arrays are not supported")]
    ArraysNotSupported,

    #[error("invalid storage value name")]
    InvalidStorageValueName(#[source] StorageValueNameError),
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

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use alloc::string::ToString;
    use core::error::Error;

    use super::*;
    use crate::account::component::toml::InitStorageDataError;

    #[test]
    fn from_toml_str_with_nested_table_and_flattened() {
        let toml_table = r#"
            [token_metadata]
            max_supply = "1000000000"
            symbol = "ETH"
            decimals = "9"
        "#;

        let toml_inline = r#"
            token_metadata.max_supply = "1000000000"
            token_metadata.symbol = "ETH"
            token_metadata.decimals = "9"
        "#;

        let storage_table = InitStorageData::from_toml(toml_table).unwrap();
        let storage_inline = InitStorageData::from_toml(toml_inline).unwrap();

        assert_eq!(storage_table.placeholders(), storage_inline.placeholders());
    }

    #[test]
    fn from_toml_str_with_deeply_nested_tables() {
        let toml_str = r#"
            [a]
            b = "0xb"

            [a.c]
            d = "0xd"

            [x.y.z]
            w = 42 # NOTE: This gets parsed as string
        "#;

        let storage = InitStorageData::from_toml(toml_str).expect("Failed to parse TOML");
        let key1 = StorageValueName::new("a.b".to_string()).unwrap();
        let key2 = StorageValueName::new("a.c.d".to_string()).unwrap();
        let key3 = StorageValueName::new("x.y.z.w".to_string()).unwrap();

        assert_eq!(storage.get(&key1).unwrap(), "0xb");
        assert_eq!(storage.get(&key2).unwrap(), "0xd");
        assert_eq!(storage.get(&key3).unwrap(), "42");
    }

    #[test]
    fn test_error_on_array() {
        let toml_str = r#"
            token_metadata.v = [1, 2, 3]
        "#;

        let result = InitStorageData::from_toml(toml_str);
        assert_matches::assert_matches!(
            result.unwrap_err(),
            InitStorageDataError::ArraysNotSupported
        );
    }

    #[test]
    fn error_on_empty_subtable() {
        let toml_str = r#"
            [a]
            b = {}
        "#;

        let result = InitStorageData::from_toml(toml_str);
        assert_matches::assert_matches!(result.unwrap_err(), InitStorageDataError::EmptyTable(_));
    }

    #[test]
    fn error_on_duplicate_keys() {
        let toml_str = r#"
            token_metadata.max_supply = "1000000000"
            token_metadata.max_supply = "500000000"
        "#;

        let result = InitStorageData::from_toml(toml_str).unwrap_err();
        // TOML does not support duplicate keys
        assert_matches::assert_matches!(result, InitStorageDataError::InvalidToml(_));
        assert!(result.source().unwrap().to_string().contains("duplicate"));
    }
}

