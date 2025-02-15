use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
};
use core::fmt::{self, Display};

use thiserror::Error;
use vm_core::{
    utils::{ByteReader, ByteWriter, Deserializable, Serializable},
    Felt, FieldElement, Word,
};
use vm_processor::DeserializationError;

use crate::{
    asset::TokenSymbol,
    utils::{parse_hex_string_as_word, sync::LazyLock},
};

/// A global registry for template converters.
///
/// It is used during component instantiation to dynamically convert template placeholders into
/// their respective storage values.
pub static TEMPLATE_REGISTRY: LazyLock<TemplateRegistry> = LazyLock::new(|| {
    let mut registry = TemplateRegistry::new();
    registry.register_felt_type::<u8>();
    registry.register_felt_type::<u16>();
    registry.register_felt_type::<u32>();
    registry.register_felt_type::<Felt>();
    registry.register_felt_type::<TokenSymbol>();
    registry.register_word_type::<u8>();
    registry.register_word_type::<u16>();
    registry.register_word_type::<u32>();
    registry.register_word_type::<Word>();
    registry.register_word_type::<FalconPubKey>();
    registry
});

// STORAGE VALUE NAME
// ================================================================================================

/// A simple wrapper type around a string key that identifies values.
///
/// A storage value name is a string that identifies dynamic values within a component's metadata
/// storage entries.
///
/// These names can be chained together, in a way that allows composing unique keys for inner
/// templated elements.
///
/// At component instantiation, a map of names to values must be provided to dynamically
/// replace these placeholders with the instance’s actual values.
#[derive(Clone, Debug, Default, Ord, PartialOrd, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(::serde::Deserialize, ::serde::Serialize))]
#[cfg_attr(feature = "std", serde(transparent))]
pub struct StorageValueName {
    fully_qualified_name: String,
}

impl StorageValueName {
    /// Creates a new [`StorageValueName`] from the provided string.
    ///
    /// A [`StorageValueName`] serves as an identifier for storage values that are determined at
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
    pub fn new(base: impl Into<String>) -> Result<Self, StorageValueNameError> {
        let base: String = base.into();
        for segment in base.split('.') {
            Self::validate_segment(segment)?;
        }
        Ok(Self { fully_qualified_name: base })
    }

    /// Adds a suffix to the storage value name, separated with a dot.
    #[must_use]
    pub fn with_suffix(self, suffix: &StorageValueName) -> StorageValueName {
        let mut key = self;
        if !suffix.as_str().is_empty() {
            if !key.as_str().is_empty() {
                key.fully_qualified_name.push('.');
            }
            key.fully_qualified_name.push_str(suffix.as_str());
        }

        key
    }

    /// Returns the fully qualified name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.fully_qualified_name
    }

    fn validate_segment(segment: &str) -> Result<(), StorageValueNameError> {
        if segment.is_empty() {
            return Err(StorageValueNameError::EmptySegment);
        }
        if let Some(offending_char) =
            segment.chars().find(|&c| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        {
            return Err(StorageValueNameError::InvalidCharacter {
                part: segment.to_string(),
                character: offending_char,
            });
        }

        Ok(())
    }
}

impl Display for StorageValueName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serializable for StorageValueName {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write(&self.fully_qualified_name);
    }
}

impl Deserializable for StorageValueName {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let key: String = source.read()?;
        Ok(StorageValueName { fully_qualified_name: key })
    }
}

#[derive(Debug, Error)]
pub enum StorageValueNameError {
    #[error("key segment is empty")]
    EmptySegment,
    #[error("key segment '{part}' contains invalid character '{character}'")]
    InvalidCharacter { part: String, character: char },
}

// TEMPLATE TYPE ERROR
// ================================================================================================

/// Errors that can occur when parsing or converting template types.
///
/// This enum covers various failure cases including parsing errors, conversion errors,
/// unsupported conversions, and cases where a required type is not found in the registry.
#[derive(Debug, Error)]
pub enum TemplateTypeError {
    #[error("failed to parse input `{0}` as `{1}`")]
    ParseError(String, String),
    #[error("conversion error: {0}")]
    ConversionError(String),
    #[error("unsupported conversion for {0}")]
    UnsupportedConversion(String),
    #[error("felt type ` {0}` not found in the type registry")]
    FeltTypeNotFound(String),
    #[error("word type ` {0}` not found in the type registry")]
    WordTypeNotFound(String),
    #[error("words type ` {0}` not found in the type registry")]
    WordsTypeNotFound(String),
}

// TEMPLATE REQUIREMENT
// ================================================================================================

/// Describes the expected type and additional metadata for a templated storage entry.
///
/// A `PlaceholderTypeRequirement` specifies the expected type identifier for a storage entry as
/// well as an optional description. This information is used to validate and provide context for
/// dynamic storage values.
#[derive(Debug)]
pub struct PlaceholderTypeRequirement {
    /// The expected type identifier as a string.
    pub r#type: String,
    /// An optional description providing additional context.
    pub description: Option<String>,
}

// TEMPLATE TRAITS
// ================================================================================================

/// Trait for converting a string into a single `Felt`.
pub trait TemplateFelt {
    /// Returns the type identifier.
    fn type_name() -> String;
    /// Parses the input string into a `Felt`.
    fn parse_felt(input: &str) -> Result<Felt, TemplateTypeError>;
}

/// Trait for converting a string into a single `Word`.
pub trait TemplateWord {
    /// Returns the type identifier.
    fn type_name() -> String;
    /// Parses the input string into a `Word`.
    fn parse_word(input: &str) -> Result<Word, TemplateTypeError>;
}

// IMPLEMENTATIONS FOR NATIVE TYPES
// ================================================================================================

impl TemplateFelt for u8 {
    fn type_name() -> String {
        "u8".into()
    }
    fn parse_felt(input: &str) -> Result<Felt, TemplateTypeError> {
        let native: u8 = input
            .parse()
            .map_err(|_| TemplateTypeError::ParseError(input.to_string(), "u8".to_string()))?;
        Ok(Felt::from(native))
    }
}

impl TemplateFelt for u16 {
    fn type_name() -> String {
        "u16".into()
    }
    fn parse_felt(input: &str) -> Result<Felt, TemplateTypeError> {
        let native: u16 = input
            .parse()
            .map_err(|_| TemplateTypeError::ParseError(input.to_string(), "u16".to_string()))?;
        Ok(Felt::from(native))
    }
}

impl TemplateFelt for u32 {
    fn type_name() -> String {
        "u32".into()
    }
    fn parse_felt(input: &str) -> Result<Felt, TemplateTypeError> {
        let native: u32 = input
            .parse()
            .map_err(|_| TemplateTypeError::ParseError(input.to_string(), "u32".to_string()))?;
        Ok(Felt::from(native))
    }
}

impl TemplateFelt for Felt {
    fn type_name() -> String {
        "felt".into()
    }

    fn parse_felt(input: &str) -> Result<Felt, TemplateTypeError> {
        let n = if let Some(hex) = input.strip_prefix("0x").or_else(|| input.strip_prefix("0X")) {
            u64::from_str_radix(hex, 16)
        } else {
            input.parse::<u64>()
        }
        .map_err(|_| TemplateTypeError::ParseError(input.to_string(), "felt".to_string()))?;
        Felt::try_from(n).map_err(|_| TemplateTypeError::ConversionError(input.to_string()))
    }
}

impl TemplateFelt for TokenSymbol {
    fn type_name() -> String {
        "tokensymbol".into()
    }
    fn parse_felt(input: &str) -> Result<Felt, TemplateTypeError> {
        let token = TokenSymbol::new(input).map_err(|_| {
            TemplateTypeError::ParseError(input.to_string(), "tokensymbol".to_string())
        })?;
        Ok(Felt::from(token))
    }
}

// WORD IMPLS
// ================================================================================================

impl TemplateWord for Word {
    fn type_name() -> String {
        "word".into()
    }
    fn parse_word(input: &str) -> Result<Word, TemplateTypeError> {
        parse_hex_string_as_word(input)
            .map_err(|e| TemplateTypeError::ParseError("word".to_string(), e.to_string()))
    }
}

#[derive(Debug, Default)]
pub struct FalconPubKey;
impl TemplateWord for FalconPubKey {
    fn type_name() -> String {
        "auth::rpo_falcon512::pub_key".into()
    }
    fn parse_word(input: &str) -> Result<Word, TemplateTypeError> {
        parse_hex_string_as_word(input).map_err(|e| {
            TemplateTypeError::ParseError("auth::rpo_falcon512::pub_key".to_string(), e.to_string())
        })
    }
}

/// Blanket implementation of `TemplateWord` for any type that implements `TemplateFelt`.
///
/// This implementation converts a value parsed as a `Felt` into a `Word` by placing the parsed
/// `Felt` in the last position of a four-element array, with the remaining elements set to zero.
impl<F> TemplateWord for F
where
    F: TemplateFelt,
{
    fn type_name() -> String {
        F::type_name()
    }

    fn parse_word(input: &str) -> Result<Word, TemplateTypeError> {
        let felt_value = F::parse_felt(input)
            .map_err(|_| TemplateTypeError::ParseError(input.into(), "word".into()))?;

        Ok([Felt::ZERO, Felt::ZERO, Felt::ZERO, felt_value])
    }
}

// TYPE ALIASES FOR CONVERTER CLOSURES
// ================================================================================================

/// Type alias for a function that converts a string into a [`Felt`] value.
type TemplateFeltConverter = fn(&str) -> Result<Felt, TemplateTypeError>;

/// Type alias for a function that converts a string into a [`Word`].
type TemplateWordConverter = fn(&str) -> Result<Word, TemplateTypeError>;

// TODO: Implement converting to list of words for multi-slot values

// TEMPLATE REGISTRY
// ================================================================================================

/// Registry for template converters.
///
/// This registry maintains mappings from type identifiers (as strings) to conversion functions for
/// [`Felt`], [`Word`], and [`Vec<Word>`] types. It is used to dynamically parse template inputs
/// into their corresponding storage representations.
#[derive(Clone, Debug, Default)]
pub struct TemplateRegistry {
    felt: BTreeMap<String, TemplateFeltConverter>,
    word: BTreeMap<String, TemplateWordConverter>,
}

impl TemplateRegistry {
    /// Creates a new, empty `TemplateRegistry`.
    ///
    /// The registry is initially empty and conversion functions can be registered using the
    /// `register_*_type` methods.
    pub fn new() -> Self {
        Self { ..Default::default() }
    }

    /// Registers a `TemplateFelt` converter, to interpret a string as a [`Felt``].
    pub fn register_felt_type<T: TemplateFelt + 'static>(&mut self) {
        let key = T::type_name();
        self.felt.insert(key, T::parse_felt);
    }

    /// Registers a `TemplateFelt` converter, to interpret a string as a [`Word`].
    pub fn register_word_type<T: TemplateWord + 'static>(&mut self) {
        let key = T::type_name();
        self.word.insert(key, T::parse_word);
    }

    /// Attempts to parse a string into a `Felt` using the registered converter for the given type
    /// name.
    ///
    /// # Arguments
    ///
    /// - type_name: A string that acts as the type identifier.
    /// - value: The string representation of the value to be parsed.
    ///
    /// # Errors
    ///
    /// - If the type is not registered or if the conversion fails.
    pub fn try_parse_felt(&self, type_name: &str, value: &str) -> Result<Felt, TemplateTypeError> {
        let converter = self
            .felt
            .get(type_name)
            .ok_or(TemplateTypeError::FeltTypeNotFound(type_name.into()))?;
        converter(value)
    }

    /// Attempts to parse a string into a `Word` using the registered converter for the given type
    /// name.
    ///
    /// # Arguments
    ///
    /// - type_name: A string that acts as the type identifier.
    /// - value: The string representation of the value to be parsed.
    ///
    /// # Errors
    ///
    /// - If the type is not registered or if the conversion fails.
    pub fn try_parse_word(&self, type_name: &str, value: &str) -> Result<Word, TemplateTypeError> {
        let converter = self
            .word
            .get(type_name)
            .ok_or(TemplateTypeError::WordTypeNotFound(type_name.into()))?;
        converter(value)
    }
}
