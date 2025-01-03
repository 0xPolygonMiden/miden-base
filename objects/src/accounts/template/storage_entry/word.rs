use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use vm_core::{
    utils::{ByteReader, ByteWriter, Deserializable, Serializable},
    Felt, FieldElement, Word,
};
use vm_processor::{DeserializationError, Digest};

use super::{TemplateKey, TemplateValue};
use crate::{accounts::template::AccountComponentTemplateError, utils::parse_hex_string_as_word};

// WORDS
// ================================================================================================

/// Supported word representations in TOML format. Represents slot values and keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordRepresentation {
    /// A word represented by a hexadecimal string.
    Hexadecimal([Felt; 4]),
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
            WordRepresentation::Dynamic(template_key) => Box::new(core::iter::once(template_key)),
            WordRepresentation::Hexadecimal(_) => Box::new(core::iter::empty()),
        }
    }

    /// Attempts to convert the [WordRepresentation] into a [Word].
    ///
    /// If the representation is dynamic, the value is retrieved from `template_values`, identified
    /// by its key. If any of the inner elements within the value are dynamic, they are retrieved
    /// in the same way.
    pub fn try_build_word(
        &self,
        template_values: &BTreeMap<String, TemplateValue>,
    ) -> Result<Word, AccountComponentTemplateError> {
        match self {
            WordRepresentation::Hexadecimal(word) => Ok(*word),
            WordRepresentation::Array(array) => {
                let mut result = [Felt::ZERO; 4];
                for (index, felt_repr) in array.iter().enumerate() {
                    result[index] = felt_repr.clone().try_build_felt(template_values)?;
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
        WordRepresentation::Hexadecimal(value)
    }
}

impl Serializable for WordRepresentation {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            WordRepresentation::Hexadecimal(value) => {
                target.write_u8(0);
                target.write(value);
            },
            WordRepresentation::Array(value) => {
                target.write_u8(1);
                target.write(value);
            },
            WordRepresentation::Dynamic(template_key) => {
                target.write_u8(2);
                target.write(template_key);
            },
        }
    }
}

impl Deserializable for WordRepresentation {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let variant_tag = source.read_u8()?;

        match variant_tag {
            0 => {
                // Hexadecimal
                let value = <[Felt; 4]>::read_from(source)?;
                Ok(WordRepresentation::Hexadecimal(value))
            },
            1 => {
                // Array
                let value = <[FeltRepresentation; 4]>::read_from(source)?;
                Ok(WordRepresentation::Array(value))
            },
            2 => {
                // Dynamic
                let template_key = TemplateKey::read_from(source)?;
                Ok(WordRepresentation::Dynamic(template_key))
            },
            _ => Err(DeserializationError::InvalidValue(format!(
                "unknown variant tag for WordRepresentation: {variant_tag}"
            ))),
        }
    }
}

#[cfg(feature = "std")]
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
            WordRepresentation::Dynamic(key) => key.serialize(serializer),
        }
    }
}

#[cfg(feature = "std")]
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
                // Attempt to deserialize as TemplateKey first
                if let Ok(tk) = TemplateKey::try_from(value) {
                    return Ok(WordRepresentation::Dynamic(tk));
                }

                // try hex parsing otherwise
                let word = parse_hex_string_as_word(value).map_err(|_err| {
                    E::invalid_value(
                        serde::de::Unexpected::Str(value),
                        &"a valid hexadecimal string or template key (in '{{key}}' format)",
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

impl Default for WordRepresentation {
    fn default() -> Self {
        WordRepresentation::Hexadecimal(Default::default())
    }
}

// FELTS
// ================================================================================================

/// Supported element representations in TOML format. Represents slot values and keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeltRepresentation {
    /// Hexadecimal representation of a field element.
    Hexadecimal(Felt),
    /// Single decimal representation of a field element.
    Decimal(Felt),
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
    pub fn try_build_felt(
        self,
        template_values: &BTreeMap<String, TemplateValue>,
    ) -> Result<Felt, AccountComponentTemplateError> {
        match self {
            FeltRepresentation::Hexadecimal(base_element) => Ok(base_element),
            FeltRepresentation::Decimal(base_element) => Ok(base_element),
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
        FeltRepresentation::Hexadecimal(Felt::default())
    }
}

impl Serializable for FeltRepresentation {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            FeltRepresentation::Hexadecimal(felt) => {
                target.write_u8(0);
                target.write(felt);
            },
            FeltRepresentation::Decimal(felt) => {
                target.write_u8(1);
                target.write(felt);
            },
            FeltRepresentation::Dynamic(template_key) => {
                target.write_u8(2);
                target.write(template_key);
            },
        }
    }
}

impl Deserializable for FeltRepresentation {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let variant_tag = source.read_u8()?;

        match variant_tag {
            0 => {
                // Hexadecimal
                let felt = Felt::read_from(source)?;
                Ok(FeltRepresentation::Hexadecimal(felt))
            },
            1 => {
                // Decimal
                let felt = Felt::read_from(source)?;
                Ok(FeltRepresentation::Decimal(felt))
            },
            2 => {
                // Dynamic
                let template_key = TemplateKey::read_from(source)?;
                Ok(FeltRepresentation::Dynamic(template_key))
            },
            _ => Err(DeserializationError::InvalidValue(format!(
                "unknown variant tag for FeltRepresentation: {}",
                variant_tag
            ))),
        }
    }
}

#[cfg(feature = "std")]
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
                Felt::try_from(decimal_value).map_err(|err| serde::de::Error::custom(err))?,
            ))
        } else if let Ok(key) = TemplateKey::try_from(&value) {
            Ok(FeltRepresentation::Dynamic(key))
        } else {
            Err(serde::de::Error::custom(
                "deserialized string value is not a valid variant of FeltRepresentation",
            ))
        }
    }
}

#[cfg(feature = "std")]
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
            FeltRepresentation::Dynamic(key) => key.serialize(serializer),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::println;

    use super::WordRepresentation;
    use crate::digest;

    #[test]
    pub fn word_repr_rt() {
        let word = WordRepresentation::Hexadecimal(digest!("0x123").into());
        let serialized = toml::to_string(&word).unwrap();

        println!("{serialized}");
    }
}
