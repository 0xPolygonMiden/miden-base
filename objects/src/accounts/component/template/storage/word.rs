use alloc::boxed::Box;

use vm_core::{
    utils::{ByteReader, ByteWriter, Deserializable, Serializable},
    Felt, FieldElement, Word,
};
use vm_processor::DeserializationError;

use super::{InitStorageData, StoragePlaceholder};
use crate::accounts::component::template::AccountComponentTemplateError;

// WORDS
// ================================================================================================

/// Supported word representations. Represents slot values and keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordRepresentation {
    /// A word represented by a hexadecimal string.
    Hexadecimal([Felt; 4]),
    /// A word represented by its four base elements.
    Array([FeltRepresentation; 4]),
    /// A placeholder value, represented as "{{key}}".
    Template(StoragePlaceholder),
}

impl WordRepresentation {
    /// Returns an iterator over all storage placeholder references within the [WordRepresentation].
    pub fn storage_placeholders(&self) -> Box<dyn Iterator<Item = &StoragePlaceholder> + '_> {
        match self {
            WordRepresentation::Array(array) => {
                Box::new(array.iter().flat_map(|felt| felt.storage_placeholders()))
            },
            WordRepresentation::Template(storage_placeholder) => {
                Box::new(core::iter::once(storage_placeholder))
            },
            WordRepresentation::Hexadecimal(_) => Box::new(core::iter::empty()),
        }
    }

    /// Attempts to convert the [WordRepresentation] into a [Word].
    ///
    /// If the representation is a template, the value is retrieved from
    /// `init_storage_data`, identified by its key. If any of the inner elements
    /// within the value are a template, they are retrieved in the same way.
    pub fn try_build_word(
        &self,
        init_storage_data: &InitStorageData,
    ) -> Result<Word, AccountComponentTemplateError> {
        match self {
            WordRepresentation::Hexadecimal(word) => Ok(*word),
            WordRepresentation::Array(array) => {
                let mut result = [Felt::ZERO; 4];
                for (index, felt_repr) in array.iter().enumerate() {
                    result[index] = felt_repr.clone().try_build_felt(init_storage_data)?;
                }
                // SAFETY: result is guaranteed to have all its 4 indices rewritten
                Ok(result)
            },
            WordRepresentation::Template(storage_placeholder) => {
                let user_value = init_storage_data
                    .get(storage_placeholder)
                    .ok_or_else(|| {
                        AccountComponentTemplateError::PlaceholderValueNotProvided(
                            storage_placeholder.clone(),
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
            WordRepresentation::Template(storage_placeholder) => {
                target.write_u8(2);
                target.write(storage_placeholder);
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
                // Template
                let storage_placeholder = StoragePlaceholder::read_from(source)?;
                Ok(WordRepresentation::Template(storage_placeholder))
            },
            _ => Err(DeserializationError::InvalidValue(format!(
                "unknown variant tag for WordRepresentation: {variant_tag}"
            ))),
        }
    }
}

impl Default for WordRepresentation {
    fn default() -> Self {
        WordRepresentation::Hexadecimal(Default::default())
    }
}

// FELTS
// ================================================================================================

/// Supported element representations for a component's storage entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeltRepresentation {
    /// Hexadecimal representation of a field element.
    Hexadecimal(Felt),
    /// Single decimal representation of a field element.
    Decimal(Felt),
    /// A placeholder value, represented as "{{key}}".
    Template(StoragePlaceholder),
}

impl FeltRepresentation {
    pub fn storage_placeholders(&self) -> impl Iterator<Item = &StoragePlaceholder> {
        let maybe_key = match self {
            FeltRepresentation::Template(storage_placeholder) => Some(storage_placeholder),
            _ => None,
        };

        maybe_key.into_iter()
    }

    /// Attempts to convert the [FeltRepresentation] into a [Felt].
    ///
    /// If the representation is a template, the value is retrieved from `init_storage_data`,
    /// identified by its key. Otherwise, the returned value is just the inner element.
    pub fn try_build_felt(
        self,
        init_storage_data: &InitStorageData,
    ) -> Result<Felt, AccountComponentTemplateError> {
        match self {
            FeltRepresentation::Hexadecimal(base_element) => Ok(base_element),
            FeltRepresentation::Decimal(base_element) => Ok(base_element),
            FeltRepresentation::Template(storage_placeholder) => init_storage_data
                .get(&storage_placeholder)
                .ok_or(
                    AccountComponentTemplateError::PlaceholderValueNotProvided(storage_placeholder))?
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
            FeltRepresentation::Template(storage_placeholder) => {
                target.write_u8(2);
                target.write(storage_placeholder);
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
                // Template
                let storage_placeholder = StoragePlaceholder::read_from(source)?;
                Ok(FeltRepresentation::Template(storage_placeholder))
            },
            _ => Err(DeserializationError::InvalidValue(format!(
                "unknown variant tag for FeltRepresentation: {}",
                variant_tag
            ))),
        }
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use vm_core::{
        utils::{Deserializable, Serializable},
        Felt, Word,
    };

    use crate::accounts::component::template::{
        storage::{FeltRepresentation, StorageValue, WordRepresentation},
        InitStorageData, StoragePlaceholder,
    };

    #[test]
    fn test_storage_placeholder_try_from_str() {
        let invalid_strings = vec![
            "{invalid}",
            "no_braces",
            "{{unclosed",
            "unopened}}",
            "{}",
            "{{}}",
            "{{.}}",
            "{{foo..bar}}",
        ];

        for s in invalid_strings {
            let result = StoragePlaceholder::try_from(s);
            result.unwrap_err();
        }

        let s = "{{storage_placeholder}}";
        let tk = StoragePlaceholder::try_from(s).unwrap();
        assert_eq!(tk.inner(), "storage_placeholder");
    }

    #[test]
    fn test_storage_placeholder_serialization_deserialization() {
        let original = StoragePlaceholder::new("serialize_test").unwrap();
        let serialized = original.to_bytes();
        let deserialized = StoragePlaceholder::read_from_bytes(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_felt_representation_serde() {
        let felt = Felt::new(1234);
        let original = FeltRepresentation::Hexadecimal(felt);
        let serialized = original.to_bytes();
        let deserialized = FeltRepresentation::read_from_bytes(&serialized).unwrap();
        assert_eq!(original, deserialized);

        let felt = Felt::new(45563);
        let original = FeltRepresentation::Decimal(felt);
        let serialized = original.to_bytes();
        let deserialized = FeltRepresentation::read_from_bytes(&serialized).unwrap();
        assert_eq!(original, deserialized);

        let storage_placeholder = StoragePlaceholder::new("template_felt").unwrap();
        let original = FeltRepresentation::Template(storage_placeholder.clone());
        let serialized = original.to_bytes();
        let deserialized = FeltRepresentation::read_from_bytes(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_felt_representation_try_build_felt() {
        let dyn_key = StoragePlaceholder::new("felt_key").unwrap();
        let template = FeltRepresentation::Template(dyn_key.clone());
        let init_storage_data = InitStorageData::new([(
            "felt_key".try_into().unwrap(),
            StorageValue::Felt(Felt::new(300)),
        )]);
        let built = template.try_build_felt(&init_storage_data).unwrap();
        assert_eq!(built, Felt::new(300));

        let dyn_key = StoragePlaceholder::new("missing_key").unwrap();
        let template = FeltRepresentation::Template(dyn_key.clone());
        let result = template.try_build_felt(&init_storage_data);
        result.unwrap_err();
    }

    #[test]
    fn test_word_representation_serde() {
        let word = Word::default();
        let original = WordRepresentation::Hexadecimal(word);
        let serialized = original.to_bytes();
        let deserialized = WordRepresentation::read_from_bytes(&serialized).unwrap();
        assert_eq!(original, deserialized);

        let array = [
            FeltRepresentation::Hexadecimal(Felt::new(10)),
            FeltRepresentation::Decimal(Felt::new(20)),
            FeltRepresentation::Template(StoragePlaceholder::new("word_key1").unwrap()),
            FeltRepresentation::Template(StoragePlaceholder::new("word_key2").unwrap()),
        ];
        let original = WordRepresentation::Array(array);
        let serialized = original.to_bytes();
        let deserialized = WordRepresentation::read_from_bytes(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_word_representation_template_serde() {
        let storage_placeholder = StoragePlaceholder::new("temlpate_word").unwrap();
        let original = WordRepresentation::Template(storage_placeholder.clone());
        let serialized = original.to_bytes();
        let deserialized = WordRepresentation::read_from_bytes(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }
}
