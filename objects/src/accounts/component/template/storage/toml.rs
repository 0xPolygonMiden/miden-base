use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use vm_core::Felt;
use vm_processor::Digest;

use super::{
    FeltRepresentation, StorageEntry, StoragePlaceholder, StorageValues, WordRepresentation,
};
use crate::{
    accounts::AccountComponentMetadata, errors::AccountComponentTemplateError,
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
        let toml = toml::to_string(self).unwrap();
        Ok(toml)
    }
}
// WORD REPRESENTATION SERIALIZATION
// ================================================================================================

impl serde::Serialize for WordRepresentation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        match self {
            WordRepresentation::Hexadecimal(word) => {
                // Ensure that the length of the vector is exactly 4
                let word = Digest::from(word);
                serializer.serialize_str(&word.to_string())
            },
            WordRepresentation::Array(words) => {
                let mut seq = serializer.serialize_seq(Some(4))?;
                for word in words {
                    seq.serialize_element(word)?;
                }
                seq.end()
            },
            WordRepresentation::Template(key) => key.serialize(serializer),
        }
    }
}

impl<'de> serde::Deserialize<'de> for WordRepresentation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error, SeqAccess, Visitor};
        struct WordRepresentationVisitor;

        impl<'de> Visitor<'de> for WordRepresentationVisitor {
            type Value = WordRepresentation;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a single hex/decimal Word or an array of 4 elements")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                // Attempt to deserialize as storage placeholder first
                if let Ok(tk) = StoragePlaceholder::try_from(value) {
                    return Ok(WordRepresentation::Template(tk));
                }

                // try hex parsing otherwise
                let word = parse_hex_string_as_word(value).map_err(|_err| {
                    E::invalid_value(
                        serde::de::Unexpected::Str(value),
                        &"a valid hexadecimal string or storage placeholder (in '{{key}}' format)",
                    )
                })?;

                Ok(WordRepresentation::Hexadecimal(word))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut elements = Vec::with_capacity(4);
                while let Some(felt_repr) = seq.next_element::<FeltRepresentation>()? {
                    elements.push(felt_repr);
                }

                if elements.len() == 4 {
                    let array: [FeltRepresentation; 4] =
                        elements.clone().try_into().map_err(|_| {
                            Error::invalid_length(
                                elements.len(),
                                &"expected an array of 4 elements",
                            )
                        })?;
                    Ok(WordRepresentation::Array(array))
                } else {
                    Err(Error::invalid_length(elements.len(), &"expected an array of 4 elements"))
                }
            }
        }

        deserializer.deserialize_any(WordRepresentationVisitor)
    }
}

// FELT REPRESENTATION SERIALIZATION
// ================================================================================================

impl<'de> serde::Deserialize<'de> for FeltRepresentation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if let Some(hex_str) = value.strip_prefix("0x").or_else(|| value.strip_prefix("0X")) {
            let felt_value = u64::from_str_radix(hex_str, 16).map_err(serde::de::Error::custom)?;
            Ok(FeltRepresentation::Hexadecimal(Felt::new(felt_value)))
        } else if let Ok(decimal_value) = value.parse::<u64>() {
            Ok(FeltRepresentation::Decimal(
                Felt::try_from(decimal_value).map_err(serde::de::Error::custom)?,
            ))
        } else if let Ok(key) = StoragePlaceholder::try_from(&value) {
            Ok(FeltRepresentation::Template(key))
        } else {
            Err(serde::de::Error::custom(
                "deserialized string value is not a valid variant of FeltRepresentation",
            ))
        }
    }
}

impl serde::Serialize for FeltRepresentation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            FeltRepresentation::Hexadecimal(felt) => {
                let output = format!("0x{:x}", felt.as_int());
                serializer.serialize_str(&output)
            },
            FeltRepresentation::Decimal(felt) => {
                let output = felt.as_int().to_string();
                serializer.serialize_str(&output)
            },
            FeltRepresentation::Template(key) => key.serialize(serializer),
        }
    }
}

// KEY SERIALIZATION
// ================================================================================================

impl serde::Serialize for StoragePlaceholder {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for StoragePlaceholder {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        StoragePlaceholder::try_from(s.as_str()).map_err(serde::de::Error::custom)
    }
}

// STORAGE ENTRY SERIALIZATION
// ================================================================================================

/// Used as a helper for validating and (de)serializing storage entries
#[derive(Default, Deserialize, Serialize)]
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
            StorageEntry::Map {
                name,
                description,
                slot,
                map_entries: values,
            } => RawStorageEntry {
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
        let value_present = raw.value.is_some();

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
                        .ok_or(D::Error::custom("missing 'slot' field for single-slot entry"))?,
                    value: raw
                        .value
                        .ok_or(D::Error::custom("missing 'value' field for single-slot entry"))?,
                })
            },
            (Some(_), None) => {
                Err(D::Error::custom("`slots` is defined but no `values` field was found"))
            },
            (None, Some(values)) => {
                // Expect a Map variant:
                //   - `slot` must be present
                //   - `values` must be present and convertible to map entries
                //   - `slots` must not be present
                //   - `value` must not be present
                if value_present {
                    return Err(D::Error::custom(
                        "fields 'value' and 'values' are mutually exclusive",
                    ));
                }

                let map_entries = values
                    .into_map_entries()
                    .ok_or_else(|| D::Error::custom("invalid 'values' for map entry"))?;

                Ok(StorageEntry::Map {
                    name: raw.name,
                    description: raw.description,
                    slot: raw.slot.ok_or(D::Error::missing_field("slot"))?,
                    map_entries,
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
                        "fields 'slot' and 'slots' are mutually exclusive",
                    ));
                }
                if value_present {
                    return Err(D::Error::custom(
                        "fields 'value' and 'values' are mutually exclusive",
                    ));
                }

                let has_list_of_values = values.is_list_of_words();
                if has_list_of_values {
                    let slots_count = slots.len();
                    let values_count = values.len().expect("checked that it's a list of values");
                    if slots_count != values_count {
                        return Err(D::Error::custom(format!(
                            "number of slots ({}) does not match number of values ({}) for multi-slot storage entry",
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
                        .ok_or_else(|| D::Error::custom("invalid values for multi-slot"))?,
                })
            },
        }
    }
}
