use crate::AccountError;

use super::{Digest, Hasher, Vec, Word};

// STORAGE ENTRY
// ================================================================================================

/// A Storage entry can be one of the following:
/// - a scalar value (e.g. a single Word)
/// - a commitment to multiple words
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum StorageEntry {
    Scalar(Word),
    Array(StorageArrayEntry),
}

impl StorageEntry {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Constructs a new [StorageEntry::Scalar] instance.
    pub fn new_scalar(word: Word) -> StorageEntry {
        StorageEntry::Scalar(word)
    }

    /// Constructs a new [StorageEntry::Array] instance.
    pub fn new_array(array: StorageArrayEntry) -> StorageEntry {
        StorageEntry::Array(array)
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the type of this value.
    pub fn entry_type(&self) -> StorageEntryType {
        match self {
            Self::Scalar(_) => StorageEntryType::Scalar,
            Self::Array(value) => value.entry_type(),
        }
    }

    /// Returns the entry that represents this value.
    pub fn value(&self) -> Word {
        match self {
            Self::Scalar(value) => *value,
            Self::Array(value) => *value.commitment(),
        }
    }
}

/// A storage entry that represents an array of words.
///
///
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct StorageArrayEntry(Vec<Word>);

impl StorageArrayEntry {
    /// Maximum allowed length of an array.
    const MAX_ARRAY_LEN: usize = 256;

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Constructs a new [ArrayEntry] instance.
    pub fn new(words: Vec<Word>) -> Result<Self, AccountError> {
        if words.len() <= 1 {
            return Err(AccountError::StorageArrayRequiresMoreThanOneElement);
        }

        if words.len() > Self::MAX_ARRAY_LEN {
            return Err(AccountError::StorageArrayTooLong {
                actual: words.len(),
                max: Self::MAX_ARRAY_LEN,
            });
        }
        Ok(Self(words))
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the type of this [ArrayValue].
    pub fn entry_type(&self) -> StorageEntryType {
        StorageEntryType::Array {
            arity: (self.0.len() - 1) as u8,
        }
    }

    /// Returns the commitment to this [ArrayValue].
    pub fn commitment(&self) -> Digest {
        Hasher::hash_elements(&self.0.concat())
    }

    /// Returns the data of this [ArrayValue].
    pub fn value(&self) -> &[Word] {
        &self.0
    }
}

/// An object that represents the type of a storage value.
///
/// A value in storage can be one of the following:
/// - a scalar value (e.g. a single Word)
/// - a commitment to multiple words
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum StorageEntryType {
    Scalar,
    Array { arity: u8 },
}

impl From<&StorageEntryType> for u8 {
    fn from(entry_type: &StorageEntryType) -> Self {
        match entry_type {
            StorageEntryType::Scalar => 0,
            StorageEntryType::Array { arity } => *arity,
        }
    }
}
