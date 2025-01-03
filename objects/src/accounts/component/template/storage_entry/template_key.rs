use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use vm_core::{
    utils::{ByteReader, ByteWriter, Deserializable, Serializable},
    Felt, Word,
};
use vm_processor::{DeserializationError, Digest};

use crate::accounts::component::template::AccountComponentTemplateError;

// TEMPLATE KEY
// ================================================================================================

/// A simple wrapper type around a string key that enables templating.
///
/// A template key is a string that identifies dynamic values within a component's metadata storage
/// entries. Template keys are serialized as "{{key}}" and can be used as placeholders in map keys,
/// map values, or individual [Felt] within a [Word].
///
/// At component instantiation, a map of keys to [TemplateValue] must be provided to dynamically
/// replace these placeholders with the instance’s actual values.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TemplateKey {
    key: String,
}

impl TemplateKey {
    /// Creates a new [TemplateKey]
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }

    /// Returns the key name
    pub fn inner(&self) -> &str {
        &self.key
    }
}

impl TryFrom<&str> for TemplateKey {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.starts_with("{{") && value.ends_with("}}") {
            let inner = &value[2..value.len() - 2];
            Ok(TemplateKey { key: inner.to_string() })
        } else {
            Err(format!("expected string in {{...}} format, got '{}'", value))
        }
    }
}

impl TryFrom<&String> for TemplateKey {
    type Error = String;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl core::fmt::Display for TemplateKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{{{{}}}}}", self.key)
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for TemplateKey {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write(&self.key);
    }
}

impl Deserializable for TemplateKey {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let key: String = source.read()?;
        Ok(TemplateKey { key })
    }
}

// SERDE SERIALIZATION
// ================================================================================================

#[cfg(feature = "std")]
impl serde::Serialize for TemplateKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(feature = "std")]
impl<'de> serde::Deserialize<'de> for TemplateKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        TemplateKey::try_from(s.as_str()).map_err(serde::de::Error::custom)
    }
}

// TEMPLATE VALUE
// ================================================================================================

/// Represents a value used within a templating context.
///
/// A [TemplateValue] can be one of:
/// - `Felt(Felt)`: a single [Felt] value
/// - `Word(Word)`: a single [Word] value
/// - `Map(Vec<(Digest, Word)>)`: alist of storage map entries, mapping [Digest] to [Word]
///
/// These values are used to resolve dynamic placeholders at component instantiation.
pub enum TemplateValue {
    Felt(Felt),
    Word(Word),
    Map(Vec<(Digest, Word)>),
}

impl TemplateValue {
    /// Returns `Some(&Felt)` if the variant is `Felt`, otherwise errors.
    pub fn as_felt(&self) -> Result<&Felt, AccountComponentTemplateError> {
        if let TemplateValue::Felt(felt) = self {
            Ok(felt)
        } else {
            Err(AccountComponentTemplateError::IncorrectTemplateValue("Felt".into()))
        }
    }

    /// Returns `Ok(&Word)` if the variant is `Word`, otherwise errors.
    pub fn as_word(&self) -> Result<&Word, AccountComponentTemplateError> {
        if let TemplateValue::Word(word) = self {
            Ok(word)
        } else {
            Err(AccountComponentTemplateError::IncorrectTemplateValue("Word".into()))
        }
    }

    /// Returns `Ok(&Vec<(Digest, Word)>>` if the variant is `Map`, otherwise errors.
    pub fn as_map(&self) -> Result<&Vec<(Digest, Word)>, AccountComponentTemplateError> {
        if let TemplateValue::Map(map) = self {
            Ok(map)
        } else {
            Err(AccountComponentTemplateError::IncorrectTemplateValue("Map".into()))
        }
    }
}
