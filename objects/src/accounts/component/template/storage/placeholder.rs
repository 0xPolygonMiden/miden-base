use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use thiserror::Error;
use vm_core::{
    utils::{ByteReader, ByteWriter, Deserializable, Serializable},
    Felt, Word,
};
use vm_processor::{DeserializationError, Digest};

use crate::accounts::component::template::AccountComponentTemplateError;

// STORAGE PLACEHOLDER
// ================================================================================================

/// A simple wrapper type around a string key that enables templating.
///
/// A storage placeholder is a string that identifies dynamic values within a component's metadata
/// storage entries. Storage placeholders are serialized as "{{key}}" and can be used as
/// placeholders in map keys, map values, or individual [Felt]s within a [Word].
///
/// At component instantiation, a map of keys to [StorageValue] must be provided to dynamically
/// replace these placeholders with the instance’s actual values.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StoragePlaceholder {
    key: String,
}

/// An identifier for the expected type for a storage placeholder.
/// These indicate which variant of [StorageValue] should be provided when instantiating a
/// component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlaceholderType {
    Felt,
    Map,
    Word,
}

impl core::fmt::Display for PlaceholderType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PlaceholderType::Felt => f.write_str("Felt"),
            PlaceholderType::Map => f.write_str("Map"),
            PlaceholderType::Word => f.write_str("Word"),
        }
    }
}

impl StoragePlaceholder {
    /// Creates a new [StoragePlaceholder] from the provided string.
    ///
    /// A [StoragePlaceholder] serves as an identifier for storage values that are determined at
    /// instantiation time of an [AccountComponentTemplate](super::super::AccountComponentTemplate).
    ///
    /// The key can consist of one or more segments separated by dots (`.`).  
    /// Each segment must be non-empty and may contain only alphanumeric characters, underscores
    /// (`_`), or hyphens (`-`).
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - Any segment (or the whole key) is empty.
    /// - Any segment contains invalid characters.
    pub fn new(key: impl Into<String>) -> Result<Self, StoragePlaceholderError> {
        let key: String = key.into();
        Self::validate(&key)?;
        Ok(Self { key })
    }

    /// Returns the key name
    pub fn inner(&self) -> &str {
        &self.key
    }

    /// Checks if the given string is a valid key.
    /// A storage placeholder is valid if it's made of one or more segments that are non-empty
    /// alphanumeric strings.
    fn validate(key: &str) -> Result<(), StoragePlaceholderError> {
        if key.is_empty() {
            return Err(StoragePlaceholderError::EmptyKey);
        }

        for segment in key.split('.') {
            if segment.is_empty() {
                return Err(StoragePlaceholderError::EmptyKey);
            }

            for c in segment.chars() {
                if !(c.is_ascii_alphanumeric() || c == '_' || c == '-') {
                    return Err(StoragePlaceholderError::InvalidChar(key.into(), c));
                }
            }
        }

        Ok(())
    }
}

impl TryFrom<&str> for StoragePlaceholder {
    type Error = StoragePlaceholderError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.starts_with("{{") && value.ends_with("}}") {
            let inner = &value[2..value.len() - 2];
            Self::validate(inner)?;

            Ok(StoragePlaceholder { key: inner.to_string() })
        } else {
            Err(StoragePlaceholderError::FormatError(value.into()))
        }
    }
}

impl TryFrom<&String> for StoragePlaceholder {
    type Error = StoragePlaceholderError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl core::fmt::Display for StoragePlaceholder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{{{{}}}}}", self.key)
    }
}

#[derive(Debug, Error)]
pub enum StoragePlaceholderError {
    #[error("entire key and key segments cannot be empty")]
    EmptyKey,
    #[error("key `{0}` is invalid (expected string in {{...}} format)")]
    FormatError(String),
    #[error(
        "key `{0}` contains invalid character ({1}) (must be alphanumeric, underscore, or hyphen)"
    )]
    InvalidChar(String, char),
}

// SERIALIZATION
// ================================================================================================

impl Serializable for StoragePlaceholder {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write(&self.key);
    }
}

impl Deserializable for StoragePlaceholder {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let key: String = source.read()?;
        StoragePlaceholder::new(key)
            .map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// STORAGE VALUE
// ================================================================================================

/// Represents a value used within a templating context.
///
/// A [StorageValue] can be one of:
/// - `Felt(Felt)`: a single [Felt] value
/// - `Word(Word)`: a single [Word] value
/// - `Map(Vec<(Digest, Word)>)`: a list of storage map entries, mapping [Digest] to [Word]
///
/// These values are used to resolve dynamic placeholders at component instantiation.
#[derive(Clone, Debug)]
pub enum StorageValue {
    Felt(Felt),
    Word(Word),
    Map(Vec<(Digest, Word)>),
}

impl StorageValue {
    /// Returns `Some(&Felt)` if the variant is `Felt`, otherwise errors.
    pub fn as_felt(&self) -> Result<&Felt, AccountComponentTemplateError> {
        if let StorageValue::Felt(felt) = self {
            Ok(felt)
        } else {
            Err(AccountComponentTemplateError::IncorrectStorageValue("Felt".into()))
        }
    }

    /// Returns `Ok(&Word)` if the variant is `Word`, otherwise errors.
    pub fn as_word(&self) -> Result<&Word, AccountComponentTemplateError> {
        if let StorageValue::Word(word) = self {
            Ok(word)
        } else {
            Err(AccountComponentTemplateError::IncorrectStorageValue("Word".into()))
        }
    }

    /// Returns `Ok(&Vec<(Digest, Word)>>` if the variant is `Map`, otherwise errors.
    pub fn as_map(&self) -> Result<&Vec<(Digest, Word)>, AccountComponentTemplateError> {
        if let StorageValue::Map(map) = self {
            Ok(map)
        } else {
            Err(AccountComponentTemplateError::IncorrectStorageValue("Map".into()))
        }
    }
}
