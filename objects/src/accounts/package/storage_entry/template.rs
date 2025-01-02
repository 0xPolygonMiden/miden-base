use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use vm_core::{Felt, Word};
use vm_processor::Digest;

use crate::accounts::package::AccountComponentTemplateError;

// TEMPLATE KEY
// ================================================================================================

// A simple wrapper type around a string key, used to enable templating.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TemplateKey {
    key: String,
}

impl TemplateKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }

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
        write!(f, "{}", self.key)
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
        TemplateKey::try_from(s.as_str()).map_err(serde::de::Error::custom)
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
