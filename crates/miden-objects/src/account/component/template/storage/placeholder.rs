use alloc::{
    boxed::Box,
    str,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    fmt::{self, Display},
    str::FromStr,
};

use thiserror::Error;
use vm_core::{
    utils::{ByteReader, ByteWriter, Deserializable, Serializable},
    Felt, FieldElement, Word,
};
use vm_processor::DeserializationError;

use crate::{asset::TokenSymbol, utils::parse_hex_string_as_word};

// CONSTANTS
// ================================================================================================

const FELT_TYPE_U8: &str = "u8";
const FELT_TYPE_U16: &str = "u16";
const FELT_TYPE_U32: &str = "u32";
const FELT_TYPE_FELT: &str = "felt";
const FELT_TYPE_TOKEN_SYMBOL: &str = "tokensymbol";

const WORD_TYPE: &str = "word";
const FALCON_PUBKEY_TYPE: &str = "auth::rpo_falcon512::pub_key";

// STORAGE VALUE NAME
// ================================================================================================

/// A simple wrapper type around a string key that identifies values.
///
/// A storage value name is a string that identifies dynamic values within a component's metadata
/// storage entries.
///
/// These names can be chained together, in a way that allows to compose unique keys for
/// inner templated elements.
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
    /// Creates a new [`StorageValueName``] from the provided string.
    ///
    /// A [`StorageValueName``] serves as an identifier for storage values that are determined at
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

// TEMPLATE REQUIREMENT
// ================================================================================================

/// Describes the expected type of value for a templated storage entry.
///
/// These types must be able to be parsed from [`String`] into the native type, and then converted
/// into the correct storage type ([`Felt`], or one or more [`Word`]s).
#[derive(Debug)]
pub struct PlaceholderTypeRequirement {
    pub r#type: Box<dyn TemplateType>,
    pub description: Option<String>,
}

// TEMPLATE TYPE
// ================================================================================================

/// The [`TemplateType`] trait defines an interface for converting strings into storage values.
///
/// Types implementing this trait support conversion from a string into either a single [`Felt`]
/// (a field element) or into one or more [`Word`]s. These conversions are used during the
/// instantiation of account component templates, where placeholder values provided as strings
/// must be parsed into their native storage representations.
pub trait TemplateType: alloc::fmt::Debug + ToString {
    /// Attempts to parse the given string into a [`Felt`].
    ///
    /// # Errors
    ///
    /// Returns a [`StorageValueError`] if the string cannot be parsed into a [`Felt`].
    fn try_parse_felt(&self, value: &str) -> Result<Felt, TemplateTypeError>;

    /// Attempts to parse the given string into a vector of [`Word`]s.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageValueError`] if the string cannot be parsed into the expected vector
    /// of [`Word`]s.
    fn try_parse_words(&self, value: &str) -> Result<Vec<Word>, TemplateTypeError>;

    /// Attempts to parse the given string into a single [`Word`].
    ///
    /// This method calls `try_parse_words` internally and returns an error if the result does
    /// not contain exactly one word.
    ///
    /// # Errors
    ///
    /// Returns a [`StorageValueError::TypeArityMismatch`] if the parsed result does not have
    /// exactly one element.
    fn try_parse_word(&self, value: &str) -> Result<Word, TemplateTypeError> {
        let mut words = self.try_parse_words(value)?;
        if words.len() != 1 {
            return Err(TemplateTypeError::TypeArityMismatch);
        }
        Ok(words.pop().expect("checked that there's one value"))
    }
}

impl TemplateType for FeltType {
    fn try_parse_felt(&self, value: &str) -> Result<Felt, TemplateTypeError> {
        self.parse_value(value)
    }

    fn try_parse_words(&self, value: &str) -> Result<Vec<Word>, TemplateTypeError> {
        let felt = self.parse_value(value)?;
        Ok(vec![[Felt::ZERO, Felt::ZERO, Felt::ZERO, felt]])
    }
}

impl TemplateType for WordType {
    fn try_parse_felt(&self, value: &str) -> Result<Felt, TemplateTypeError> {
        match self {
            WordType::FeltType(ft) => ft.try_parse_felt(value),
            _ => Err(TemplateTypeError::TypeArityMismatch),
        }
    }

    fn try_parse_words(&self, value: &str) -> Result<Vec<Word>, TemplateTypeError> {
        match self {
            WordType::FeltType(ft) => {
                let felt = ft.parse_value(value)?;
                Ok(vec![[Felt::ZERO, Felt::ZERO, Felt::ZERO, felt]])
            },
            WordType::Words(1) => {
                let word = parse_hex_string_as_word(value).map_err(|e| {
                    TemplateTypeError::ParseError(WordType::Words(1).to_string(), e.to_string())
                })?;
                Ok(vec![word])
            },
            _ => todo!("No native types for multi-slot values are yet implemented"),
        }
    }
}

// FELT TYPE
// ================================================================================================

/// Describes native types that fit within a single Felt.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(try_from = "String", into = "String"))]
#[repr(u8)]
pub enum FeltType {
    U8,
    U16,
    U32,
    #[default]
    Felt,
    TokenSymbol,
}

impl FeltType {
    pub fn parse_value(&self, value_str: &str) -> Result<Felt, TemplateTypeError> {
        let felt = match self {
            FeltType::U8 => Felt::from(value_str.parse::<u8>().map_err(|_| {
                TemplateTypeError::ParseError(value_str.to_string(), self.to_string())
            })?),
            FeltType::U16 => Felt::from(value_str.parse::<u16>().map_err(|_| {
                TemplateTypeError::ParseError(value_str.to_string(), self.to_string())
            })?),
            FeltType::U32 => Felt::from(value_str.parse::<u32>().map_err(|_| {
                TemplateTypeError::ParseError(value_str.to_string(), self.to_string())
            })?),
            FeltType::Felt => parse_felt_from_str(value_str).map_err(|_| {
                TemplateTypeError::ParseError(
                    FeltType::TokenSymbol.to_string(),
                    value_str.to_string(),
                )
            })?,
            FeltType::TokenSymbol => Felt::from(TokenSymbol::new(value_str).map_err(|_| {
                TemplateTypeError::ParseError(FeltType::TokenSymbol.to_string(), self.to_string())
            })?),
        };
        Ok(felt)
    }
}

impl Serializable for FeltType {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_u8(*self as u8);
    }
}

impl Deserializable for FeltType {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let tag = source.read_u8()?;
        FeltType::try_from(tag).map_err(|_| {
            DeserializationError::InvalidValue(format!("unknown tag {} for FeltType", tag))
        })
    }
}

impl TryFrom<u8> for FeltType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FeltType::U8),
            1 => Ok(FeltType::U16),
            2 => Ok(FeltType::U32),
            3 => Ok(FeltType::Felt),
            4 => Ok(FeltType::TokenSymbol),
            _ => Err(()),
        }
    }
}

impl fmt::Display for FeltType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FeltType::U8 => write!(f, "{}", FELT_TYPE_U8),
            FeltType::U16 => write!(f, "{}", FELT_TYPE_U16),
            FeltType::U32 => write!(f, "{}", FELT_TYPE_U32),
            FeltType::Felt => write!(f, "{}", FELT_TYPE_FELT),
            FeltType::TokenSymbol => write!(f, "{}", FELT_TYPE_TOKEN_SYMBOL),
        }
    }
}

impl FromStr for FeltType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            x if x == FELT_TYPE_U8 => Ok(FeltType::U8),
            x if x == FELT_TYPE_U16 => Ok(FeltType::U16),
            x if x == FELT_TYPE_U32 => Ok(FeltType::U32),
            x if x == FELT_TYPE_FELT => Ok(FeltType::Felt),
            x if x == FELT_TYPE_TOKEN_SYMBOL => Ok(FeltType::TokenSymbol),
            _ => Err(String::from("invalid felt type: ") + s),
        }
    }
}

impl TryFrom<String> for FeltType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse::<FeltType>()
    }
}

impl From<FeltType> for String {
    fn from(ft: FeltType) -> Self {
        ft.to_string()
    }
}

// WORD TYPE
// ================================================================================================

/// Describes native types that fit within a certain amount of words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "std", serde(try_from = "String", into = "String"))]
pub enum WordType {
    Words(u8),
    RpoFalcon512PublicKey,
    FeltType(FeltType),
}

impl fmt::Display for WordType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WordType::Words(1) => f.write_str(WORD_TYPE),
            WordType::Words(n) => write!(f, "[word;{}]", n),
            WordType::FeltType(ft) => write!(f, "{}", ft),
            WordType::RpoFalcon512PublicKey => f.write_str(FALCON_PUBKEY_TYPE),
        }
    }
}

impl FromStr for WordType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if let (Some(inner), Some(_)) = (s.strip_prefix("[word;"), s.strip_suffix("]")) {
            let num_str = inner.trim();
            if num_str.is_empty() {
                return Err("missing number for word type".into());
            }
            return num_str
                .parse::<u8>()
                .map(WordType::Words)
                .map_err(|_| format!("invalid number in word type: {}", s));
        }

        if s == FALCON_PUBKEY_TYPE {
            return Ok(WordType::RpoFalcon512PublicKey);
        }

        if s == WORD_TYPE {
            return Ok(WordType::Words(1));
        }

        s.parse::<FeltType>()
            .map(WordType::FeltType)
            .map_err(|e| format!("failed to parse as FeltType: {}", e))
    }
}

impl Serializable for WordType {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            WordType::Words(n) => {
                target.write_u8(0);
                target.write_u8(*n);
            },
            WordType::FeltType(ft) => {
                target.write_u8(1);
                target.write(ft);
            },
            WordType::RpoFalcon512PublicKey => {
                target.write_u8(2);
            },
        }
    }
}

impl Deserializable for WordType {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        match source.read_u8()? {
            0 => Ok(WordType::Words(source.read_u8()?)),
            1 => Ok(WordType::FeltType(FeltType::read_from(source)?)),
            2 => Ok(WordType::RpoFalcon512PublicKey),
            tag => {
                Err(DeserializationError::InvalidValue(format!("unknown tag {} for WordType", tag)))
            },
        }
    }
}

impl TryFrom<String> for WordType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse::<WordType>()
    }
}

impl From<WordType> for String {
    fn from(value: WordType) -> Self {
        value.to_string()
    }
}

/// PLACEHOLDER VALUE
// ================================================================================================

#[derive(Debug, Error)]
pub enum TemplateTypeError {
    #[error("failed to convert type into felt: {0}")]
    ConversionError(String),
    #[error("failed to parse string `{0}` as `{1}`")]
    ParseError(String, String),
    #[error("parsed value does not fit into expected slot")]
    TypeArityMismatch,
}

// HELPERS
// ================================================================================================

pub(crate) fn parse_felt_from_str(s: &str) -> Result<Felt, String> {
    let n = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16)
    } else {
        s.parse::<u64>()
    }
    .map_err(|e| e.to_string())?;
    Felt::try_from(n).map_err(|e| e.to_string())
}
