use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use serde::{
    de::{Error as DeError, SeqAccess, Unexpected, Visitor},
    ser::SerializeSeq,
    Deserialize, Deserializer, Serialize, Serializer,
};
use vm_core::{Felt, FieldElement, Word};
use vm_processor::Digest;

use super::{TemplateKey, TemplateValue};
use crate::{accounts::package::AccountComponentTemplateError, utils::parse_hex_string_as_word};

// WORDS
// ================================================================================================

/// Supported word representations in TOML format. Represents slot values and keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordRepresentation {
    /// A word represented by a hexadecimal string.
    SingleHex([Felt; 4]),
    /// A word represented by its four base elements.
    Array([FeltRepresentation; 4]),
    /// A template written as "{{key}}".
    Dynamic(TemplateKey),
}

impl WordRepresentation {
    /// Returns an iterator over all `TemplateKey` references within the `WordRepresentation`.
    pub fn template_keys(&self) -> Box<dyn Iterator<Item = &TemplateKey> + '_> {
        match self {
            WordRepresentation::Array(array) => {
                Box::new(array.iter().flat_map(|felt| felt.template_keys()))
            },
            WordRepresentation::Dynamic(template_key) => Box::new(std::iter::once(template_key)),
            WordRepresentation::SingleHex(_) => Box::new(std::iter::empty()),
        }
    }

    /// Attempts to convert the [WordRepresentation] into a [Word].
    ///
    /// If the representation is dynamic, the value is retrieved from `template_values`, identified
    /// by its key. If any of the inner elements within the value are dynamic, they are retrieved
    /// in the same way.
    pub fn try_into_word(
        self,
        template_values: &BTreeMap<String, TemplateValue>,
    ) -> Result<Word, AccountComponentTemplateError> {
        match self {
            WordRepresentation::SingleHex(word) => Ok(word),
            WordRepresentation::Array(array) => {
                let mut result = [Felt::ZERO; 4];
                for (index, felt_repr) in array.into_iter().enumerate() {
                    result[index] = felt_repr.try_into_felt(template_values)?;
                }
                Ok(result)
            },
            WordRepresentation::Dynamic(template_key) => {
                let user_value = template_values
                    .get(template_key.inner())
                    .ok_or_else(|| {
                        AccountComponentTemplateError::TemplateValueNotProvided(
                            template_key.inner().to_string(),
                        )
                    })?
                    .as_word()?;
                Ok(*user_value)
            },
        }
    }
}

impl From<Word> for WordRepresentation {
    fn from(value: Word) -> Self {
        WordRepresentation::SingleHex(value)
    }
}

impl Serialize for WordRepresentation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            WordRepresentation::SingleHex(word) => {
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
            WordRepresentation::Dynamic(key) => key.serialize(serializer),
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
                formatter.write_str("a single hex/decimal Word or an array of 4 elements")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: DeError,
            {
                // Attempt to deserialize as TemplateKey first
                if let Ok(tk) = TemplateKey::try_from(value) {
                    return Ok(WordRepresentation::Dynamic(tk));
                }

                // try hex parsing otherwise
                let word = parse_hex_string_as_word(value).map_err(|_err| {
                    E::invalid_value(
                        Unexpected::Str(value),
                        &"a valid hexadecimal string or template key (in '{{key}}' format)",
                    )
                })?;

                Ok(WordRepresentation::SingleHex(word))
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
                            DeError::invalid_length(
                                elements.len(),
                                &"expected an array of 4 elements",
                            )
                        })?;
                    Ok(WordRepresentation::Array(array))
                } else {
                    Err(DeError::invalid_length(elements.len(), &"expected an array of 4 elements"))
                }
            }
        }

        deserializer.deserialize_any(WordRepresentationVisitor)
    }
}

impl Default for WordRepresentation {
    fn default() -> Self {
        WordRepresentation::SingleHex(Default::default())
    }
}

// FELTS
// ================================================================================================

/// Supported element representations in TOML format. Represents slot values and keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeltRepresentation {
    /// Hexadecimal representation of a field element.
    SingleHex(Felt),
    /// Single decimal representation of a field element.
    SingleDecimal(Felt),
    /// A template key written as "{{key}}".
    Dynamic(TemplateKey),
}

impl FeltRepresentation {
    pub fn template_keys(&self) -> impl Iterator<Item = &TemplateKey> {
        let maybe_key = match self {
            FeltRepresentation::Dynamic(template_key) => Some(template_key),
            _ => None,
        };

        maybe_key.into_iter()
    }

    /// Attempts to convert the [FeltRepresentation] into a [Felt].
    ///
    /// If the representation is dynamic, the value is retrieved from `template_values`, identified
    /// by its key. Otherwise, the returned value is just the inner element.
    pub fn try_into_felt(
        self,
        template_values: &BTreeMap<String, TemplateValue>,
    ) -> Result<Felt, AccountComponentTemplateError> {
        match self {
            FeltRepresentation::SingleHex(base_element) => Ok(base_element),
            FeltRepresentation::SingleDecimal(base_element) => Ok(base_element),
            FeltRepresentation::Dynamic(template_key) => template_values
                .get(template_key.inner())
                .ok_or_else(|| {
                    AccountComponentTemplateError::TemplateValueNotProvided(
                        template_key.inner().to_string(),
                    )
                })?
                .as_felt()
                .copied(),
        }
    }
}

impl Default for FeltRepresentation {
    fn default() -> Self {
        FeltRepresentation::SingleHex(Felt::default())
    }
}

impl<'de> Deserialize<'de> for FeltRepresentation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if let Some(hex_str) = value.strip_prefix("0x").or_else(|| value.strip_prefix("0X")) {
            let felt_value = u64::from_str_radix(hex_str, 16).map_err(serde::de::Error::custom)?;
            Ok(FeltRepresentation::SingleHex(Felt::new(felt_value)))
        } else if let Ok(decimal_value) = value.parse::<u64>() {
            Ok(FeltRepresentation::SingleDecimal(Felt::new(decimal_value)))
        } else if let Ok(key) = TemplateKey::try_from(&value) {
            Ok(FeltRepresentation::Dynamic(key))
        } else {
            Err(serde::de::Error::custom("Value is not a valid element"))
        }
    }
}

impl Serialize for FeltRepresentation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            FeltRepresentation::SingleHex(felt) => {
                let output = format!("0x{:x}", felt.as_int());
                serializer.serialize_str(&output)
            },
            FeltRepresentation::SingleDecimal(felt) => {
                let output = felt.as_int().to_string();
                serializer.serialize_str(&output)
            },
            FeltRepresentation::Dynamic(key) => key.serialize(serializer),
        }
    }
}
