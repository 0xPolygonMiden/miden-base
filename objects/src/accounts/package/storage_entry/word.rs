use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use serde::{
    de::{Error as DeError, SeqAccess, Unexpected, Visitor},
    ser::SerializeSeq,
    Deserialize, Deserializer, Serialize, Serializer,
};
use vm_core::{Felt, Word};
use vm_processor::Digest;

use crate::utils::parse_hex_string_as_word;

// WORDS
// ================================================================================================

/// Supported word representations in TOML format. Represents slot values and keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordRepresentation {
    /// A word represented by a hexadecimal string
    SingleHex([Felt; 4]),
    /// A word represented by its four base elements
    Array([FeltRepresentation; 4]),
}

impl From<Word> for WordRepresentation {
    fn from(value: Word) -> Self {
        WordRepresentation::SingleHex(value.into())
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
                // Attempt to convert the input string to a Digest
                let word = parse_hex_string_as_word(value).map_err(|_err| {
                    E::invalid_value(Unexpected::Str(value), &"a valid hexadecimal string")
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

// FELTS
// ================================================================================================

/// Supported element representations in TOML format. Represents slot values and keys.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FeltRepresentation {
    SingleHex(Felt),
    SingleDecimal(Felt),
}

impl From<FeltRepresentation> for Felt {
    fn from(val: FeltRepresentation) -> Self {
        match val {
            FeltRepresentation::SingleHex(base_element) => base_element,
            FeltRepresentation::SingleDecimal(base_element) => base_element,
        }
    }
}

impl Default for FeltRepresentation {
    fn default() -> Self {
        FeltRepresentation::SingleHex(Felt::default())
    }
}

impl Default for WordRepresentation {
    fn default() -> Self {
        WordRepresentation::SingleHex(Default::default())
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
        }
    }
}
