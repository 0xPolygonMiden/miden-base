use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use vm_core::{Felt, Word};
use vm_processor::Digest;

use crate::accounts::package::ComponentPackageError;

// TEMPLATE KEY
// ================================================================================================

// A simple wrapper type around a string key, used to enable templating.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TemplateKey {
    key: String,
}

impl TemplateKey {
    pub fn new(key: String) -> Self {
        Self { key }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub(crate) fn try_deserialize(value: &str) -> Result<TemplateKey, String> {
        if value.starts_with("{{") && value.ends_with("}}") {
            let inner = &value[2..value.len() - 2];
            Ok(TemplateKey { key: inner.to_string() })
        } else {
            Err(format!("expected string in {{...}} format, got '{}'", value))
        }
    }
}

impl From<&str> for TemplateKey {
    fn from(value: &str) -> Self {
        TemplateKey::new(value.to_string())
    }
}

impl Serialize for TemplateKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Wrap the key in "{{" and "}}"
        let wrapped = format!("{{{{{}}}}}", self.key);
        serializer.serialize_str(&wrapped)
    }
}

impl<'de> Deserialize<'de> for TemplateKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        TemplateKey::try_deserialize(&s).map_err(serde::de::Error::custom)
    }
}

// TEMPLATE VALUE
// ================================================================================================

pub enum TemplateValue {
    Felt(Felt),
    Word(Word),
    Map(Vec<(Digest, Word)>),
}

impl TemplateValue {
    /// Returns `Some(&Felt)` if the variant is `Felt`, otherwise errors.
    pub fn as_felt(&self) -> Result<&Felt, ComponentPackageError> {
        if let TemplateValue::Felt(felt) = self {
            Ok(felt)
        } else {
            Err(ComponentPackageError::IncorrectTemplateValue("Felt".into()))
        }
    }

    /// Returns `Ok(&Word)` if the variant is `Word`, otherwise errors.
    pub fn as_word(&self) -> Result<&Word, ComponentPackageError> {
        if let TemplateValue::Word(word) = self {
            Ok(word)
        } else {
            Err(ComponentPackageError::IncorrectTemplateValue("Word".into()))
        }
    }

    /// Returns `Ok(&Vec<(Digest, Word)>>` if the variant is `Map`, otherwise errors.
    pub fn as_map(&self) -> Result<&Vec<(Digest, Word)>, ComponentPackageError> {
        if let TemplateValue::Map(map) = self {
            Ok(map)
        } else {
            Err(ComponentPackageError::IncorrectTemplateValue("Map".into()))
        }
    }
}
