use alloc::{
    boxed::Box,
    collections::BTreeSet,
    string::{String, ToString},
    vec::Vec,
};
use core::iter;

use super::{
    placeholder::{PlaceholderTypeRequirement, TEMPLATE_REGISTRY},
    InitStorageData, MapEntry, StorageValueName, TemplateRequirementsIter,
};
use crate::{
    account::{component::template::AccountComponentTemplateError, StorageMap},
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest, Felt, FieldElement, Word,
};

// WORDS
// ================================================================================================

/// Defines how a word is represented within the component's storage description.
///
/// Each word representation can be:
/// - A template that defines a type but does not carry a value.
/// - A predefined value that may contain a hardcoded word or a mix of fixed and templated felts.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum WordRepresentation {
    /// A templated value that serves as a placeholder for instantiation.
    ///
    /// This variant defines a type but does not store a value. The actual value is provided at the
    /// time of instantiation. The name is required to identify this template externally.
    Template {
        /// The type associated with this templated word.
        r#type: String,
        /// A human-readable identifier for the template.
        name: StorageValueName,
        /// An optional description explaining the purpose of this template.
        description: Option<String>,
    },

    /// A predefined value that can be used directly within storage.
    ///
    /// This variant may contain either a fully hardcoded word or a structured set of felts, some
    /// of which may themselves be templates.
    Value {
        /// An optional name to identify this value externally.
        name: Option<StorageValueName>,
        /// An optional description explaining the role of this value.
        description: Option<String>,
        /// The 4-felt representation of the stored word.
        value: [FeltRepresentation; 4],
    },
}

impl WordRepresentation {
    /// Constructs a new `Template` variant.
    pub fn new_template(
        r#type: impl Into<String>,
        name: StorageValueName,
        description: Option<String>,
    ) -> Self {
        WordRepresentation::Template { name, description, r#type: r#type.into() }
    }

    /// Constructs a new `Value` variant.
    pub fn new_value(
        value: impl Into<[FeltRepresentation; 4]>,
        name: Option<StorageValueName>,
        description: Option<String>,
    ) -> Self {
        WordRepresentation::Value { name, description, value: value.into() }
    }

    /// Returns the name associated with the word representation.
    /// - For the `Template` variant, it always returns a reference to the name.
    /// - For the `Value` variant, it returns `Some` if a name is present, or `None` otherwise.
    pub fn name(&self) -> Option<&StorageValueName> {
        match self {
            WordRepresentation::Template { name, .. } => Some(name),
            WordRepresentation::Value { name, .. } => name.as_ref(),
        }
    }

    /// Returns the description associated with the word representation.
    /// Both variants store an `Option<String>`, which is converted to an `Option<&str>`.
    pub fn description(&self) -> Option<&str> {
        match self {
            WordRepresentation::Template { description, .. } => description.as_deref(),
            WordRepresentation::Value { description, .. } => description.as_deref(),
        }
    }

    /// Returns the type name.
    pub fn word_type(&self) -> &str {
        match self {
            WordRepresentation::Template { r#type, .. } => r#type,
            WordRepresentation::Value { .. } => "word",
        }
    }

    /// Returns the value (an array of 4 `FeltRepresentation`s) if this is a `Value`
    /// variant; otherwise, returns `None`.
    pub fn value(&self) -> Option<&[FeltRepresentation; 4]> {
        match self {
            WordRepresentation::Value { value, .. } => Some(value),
            WordRepresentation::Template { .. } => None,
        }
    }

    /// Returns an iterator over the word's placeholders.
    ///
    /// For [`WordRepresentation::Value`], it corresponds to the inner iterators (since inner
    /// elements can be templated as well).
    /// For [`WordRepresentation::Template`] it returns the words's placeholder requirements
    /// as defined.
    pub fn template_requirements(
        &self,
        placeholder_prefix: StorageValueName,
    ) -> TemplateRequirementsIter<'_> {
        let placeholder_key =
            placeholder_prefix.with_suffix(self.name().unwrap_or(&StorageValueName::default()));
        match self {
            // If it's a template, return the corresponding requirements
            WordRepresentation::Template { description, r#type, .. } => Box::new(iter::once((
                placeholder_key,
                PlaceholderTypeRequirement {
                    description: description.clone(),
                    r#type: r#type.clone(),
                },
            ))),
            // Otherwise, return inner iterators
            WordRepresentation::Value { value, .. } => Box::new(
                value
                    .iter()
                    .flat_map(move |felt| felt.template_requirements(placeholder_key.clone())),
            ),
        }
    }

    /// Attempts to convert the [WordRepresentation] into a [Word].
    ///
    /// If the representation is a template, the value is retrieved from
    /// `init_storage_data`, identified by its key. If any of the inner elements
    /// within the value are a template, they are retrieved in the same way.
    pub(crate) fn try_build_word(
        &self,
        init_storage_data: &InitStorageData,
        placeholder_prefix: StorageValueName,
    ) -> Result<Word, AccountComponentTemplateError> {
        match self {
            WordRepresentation::Template { name: placeholder_key, r#type, .. } => {
                let placeholder_path = placeholder_prefix.with_suffix(placeholder_key);
                let maybe_value = init_storage_data.get(&placeholder_path);
                if let Some(value) = maybe_value {
                    let parsed_value = TEMPLATE_REGISTRY
                        .try_parse_word(r#type, value)
                        .map_err(AccountComponentTemplateError::StorageValueParsingError)?;

                    Ok(parsed_value)
                } else {
                    Err(AccountComponentTemplateError::PlaceholderValueNotProvided(
                        placeholder_path,
                    ))
                }
            },
            WordRepresentation::Value { value, name, .. } => {
                let mut result = [Felt::ZERO; 4];

                for (index, felt_repr) in value.iter().enumerate() {
                    let placeholder =
                        placeholder_prefix.clone().with_suffix(&name.clone().unwrap_or_default());
                    result[index] = felt_repr.try_build_felt(init_storage_data, placeholder)?;
                }
                // SAFETY: result is guaranteed to have all its 4 indices rewritten
                Ok(result)
            },
        }
    }

    /// Validates that the defined type exists and all the inner felt types exist as well
    pub(crate) fn validate(&self) -> Result<(), AccountComponentTemplateError> {
        // Check that type exists in registry
        let type_exists = TEMPLATE_REGISTRY.contains_word_type(self.word_type());
        if !type_exists {
            return Err(AccountComponentTemplateError::InvalidType(
                self.word_type().to_string(),
                "Word".into(),
            ));
        }

        if let Some(felts) = self.value() {
            for felt in felts {
                felt.validate()?;
            }
        }

        Ok(())
    }
}

impl Serializable for WordRepresentation {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            WordRepresentation::Template { name, description, r#type } => {
                target.write_u8(0);
                target.write(name);
                target.write(description);
                target.write(r#type);
            },
            WordRepresentation::Value { name, description, value } => {
                target.write_u8(1);
                target.write(name);
                target.write(description);
                target.write(value);
            },
        }
    }
}

impl Deserializable for WordRepresentation {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let tag = source.read_u8()?;
        match tag {
            0 => {
                let name = StorageValueName::read_from(source)?;
                let description = Option::<String>::read_from(source)?;
                let r#type = String::read_from(source)?;
                Ok(WordRepresentation::Template { name, description, r#type })
            },
            1 => {
                let name = Option::<StorageValueName>::read_from(source)?;
                let description = Option::<String>::read_from(source)?;
                let value = <[FeltRepresentation; 4]>::read_from(source)?;
                Ok(WordRepresentation::Value { name, description, value })
            },
            other => Err(DeserializationError::InvalidValue(format!(
                "Unknown tag for WordRepresentation: {}",
                other
            ))),
        }
    }
}

impl From<[FeltRepresentation; 4]> for WordRepresentation {
    fn from(value: [FeltRepresentation; 4]) -> Self {
        WordRepresentation::new_value(
            value,
            Option::<StorageValueName>::None,
            Option::<String>::None,
        )
    }
}

impl From<[Felt; 4]> for WordRepresentation {
    fn from(value: [Felt; 4]) -> Self {
        WordRepresentation::new_value(
            value.map(FeltRepresentation::from),
            Option::<StorageValueName>::None,
            Option::<String>::None,
        )
    }
}

// FELTS
// ================================================================================================

/// Supported element representations for a component's storage entries.
///
/// Each felt element in a storage entry can either be:
/// - A concrete value that holds a predefined felt.
/// - A template that specifies the type of felt expected, with the actual value to be provided
///   later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeltRepresentation {
    /// A concrete felt value.
    ///
    /// This variant holds a felt that is part of the component's storage.
    /// The optional name allows for identification, and the description offers additional context.
    Value {
        /// An optional identifier for this felt value.
        name: Option<StorageValueName>,
        /// An optional explanation of the felt's purpose.
        description: Option<String>,
        /// The actual felt value.
        value: Felt,
    },

    /// A templated felt element.
    ///
    /// This variant specifies the expected type of the felt without providing a concrete value.
    /// The name is required to uniquely identify the template, and an optional description can
    /// further clarify its intended use.
    Template {
        /// The expected type for this felt element.
        r#type: String,
        /// A unique name for the felt template.
        name: StorageValueName,
        /// An optional description that explains the purpose of this template.
        description: Option<String>,
    },
}

impl FeltRepresentation {
    /// Creates a new [`FeltRepresentation::Value`] variant.
    pub fn new_value(
        value: impl Into<Felt>,
        name: Option<StorageValueName>,
        description: Option<String>,
    ) -> FeltRepresentation {
        FeltRepresentation::Value { value: value.into(), name, description }
    }

    /// Creates a new [`FeltRepresentation::Template`] variant.
    ///
    /// The name will be used for identification at the moment of instantiating the componentn.
    pub fn new_template(
        r#type: impl Into<String>,
        name: StorageValueName,
        description: Option<String>,
    ) -> FeltRepresentation {
        FeltRepresentation::Template { name, description, r#type: r#type.into() }
    }

    /// Returns the type name.
    pub fn felt_type(&self) -> &str {
        match self {
            FeltRepresentation::Template { r#type, .. } => r#type,
            FeltRepresentation::Value { .. } => "felt",
        }
    }

    /// Attempts to convert the [FeltRepresentation] into a [Felt].
    ///
    /// If the representation is a template, the value is retrieved from `init_storage_data`,
    /// identified by its key. Otherwise, the returned value is just the inner element.
    pub(crate) fn try_build_felt(
        &self,
        init_storage_data: &InitStorageData,
        placeholder_prefix: StorageValueName,
    ) -> Result<Felt, AccountComponentTemplateError> {
        match self {
            FeltRepresentation::Template { name, r#type, .. } => {
                let placeholder_key = placeholder_prefix.with_suffix(name);
                let raw_value = init_storage_data.get(&placeholder_key).ok_or(
                    AccountComponentTemplateError::PlaceholderValueNotProvided(placeholder_key),
                )?;

                Ok(TEMPLATE_REGISTRY
                    .try_parse_felt(r#type, raw_value)
                    .map_err(AccountComponentTemplateError::StorageValueParsingError)?)
            },
            FeltRepresentation::Value { value, .. } => Ok(*value),
        }
    }

    /// Returns an iterator over the felt's template.
    ///
    /// For [`FeltRepresentation::Value`], these is an empty set; for
    /// [`FeltRepresentation::Template`] it returns the felt's placeholder key based on the
    /// felt's name within the component description.
    pub fn template_requirements(
        &self,
        placeholder_prefix: StorageValueName,
    ) -> TemplateRequirementsIter<'_> {
        match self {
            FeltRepresentation::Template { name, description, r#type } => Box::new(iter::once((
                placeholder_prefix.with_suffix(name),
                PlaceholderTypeRequirement {
                    description: description.clone(),
                    r#type: r#type.clone(),
                },
            ))),
            _ => Box::new(iter::empty()),
        }
    }

    /// Validates that the defined Felt type exists
    pub(crate) fn validate(&self) -> Result<(), AccountComponentTemplateError> {
        // Check that type exists in registry
        let type_exists = TEMPLATE_REGISTRY.contains_felt_type(self.felt_type());
        if !type_exists {
            return Err(AccountComponentTemplateError::InvalidType(
                self.felt_type().to_string(),
                "Felt".into(),
            ));
        }
        Ok(())
    }
}

impl From<Felt> for FeltRepresentation {
    fn from(value: Felt) -> Self {
        FeltRepresentation::new_value(
            value,
            Option::<StorageValueName>::None,
            Option::<String>::None,
        )
    }
}

impl Default for FeltRepresentation {
    fn default() -> Self {
        FeltRepresentation::new_value(
            Felt::default(),
            Option::<StorageValueName>::None,
            Option::<String>::None,
        )
    }
}

impl Serializable for FeltRepresentation {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            FeltRepresentation::Value { name, description, value } => {
                target.write_u8(0);
                target.write(name);
                target.write(description);
                target.write(value);
            },
            FeltRepresentation::Template { name, description, r#type } => {
                target.write_u8(1);
                target.write(name);
                target.write(description);
                target.write(r#type);
            },
        }
    }
}

impl Deserializable for FeltRepresentation {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let tag = source.read_u8()?;
        match tag {
            0 => {
                let name = Option::<StorageValueName>::read_from(source)?;
                let description = Option::<String>::read_from(source)?;
                let value = Felt::read_from(source)?;
                Ok(FeltRepresentation::new_value(value, name, description))
            },
            1 => {
                let name = StorageValueName::read_from(source)?;
                let description = Option::<String>::read_from(source)?;
                let r#type = String::read_from(source)?;
                Ok(FeltRepresentation::new_template(r#type, name, description))
            },
            other => Err(DeserializationError::InvalidValue(format!(
                "Unknown tag for FeltRepresentation: {}",
                other
            ))),
        }
    }
}

// MAP REPRESENTATION
// ================================================================================================

/// Supported map representations for a component's storage entries.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(::serde::Deserialize, ::serde::Serialize))]
pub struct MapRepresentation {
    /// The human-readable name of the map slot.
    name: StorageValueName,
    /// An optional description for the slot, explaining its purpose.
    description: Option<String>,
    /// Storage map entries, consisting of a list of keys associated with their values.
    entries: Vec<MapEntry>,
}

impl MapRepresentation {
    /// Creates a new `MapRepresentation` from a vector of map entries.
    pub fn new(
        entries: Vec<MapEntry>,
        name: impl Into<StorageValueName>,
        description: Option<String>,
    ) -> Self {
        Self { entries, name: name.into(), description }
    }

    /// Returns an iterator over all of the storage entries' placeholder keys, alongside their
    /// expected type.
    pub fn template_requirements(&self) -> TemplateRequirementsIter<'_> {
        Box::new(
            self.entries
                .iter()
                .flat_map(move |entry| entry.template_requirements(self.name.clone())),
        )
    }

    /// Returns a reference to map entries.
    pub fn entries(&self) -> &[MapEntry] {
        &self.entries
    }

    /// Returns a reference to the map's name within the storage metadata.
    pub fn name(&self) -> &StorageValueName {
        &self.name
    }

    /// Returns a reference to the field's description.
    pub fn description(&self) -> Option<&String> {
        self.description.as_ref()
    }

    /// Returns the number of key-value pairs in the map.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if there are no entries in the map.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Attempts to convert the [MapRepresentation] into a [StorageMap].
    ///
    /// If any of the inner elements are templates, their values are retrieved from
    /// `init_storage_data`, identified by their key.
    pub fn try_build_map(
        &self,
        init_storage_data: &InitStorageData,
    ) -> Result<StorageMap, AccountComponentTemplateError> {
        let entries = self
            .entries
            .iter()
            .map(|map_entry| {
                let key = map_entry.key().try_build_word(init_storage_data, self.name().clone())?;
                let value =
                    map_entry.value().try_build_word(init_storage_data, self.name().clone())?;
                Ok((key.into(), value))
            })
            .collect::<Result<Vec<(Digest, Word)>, _>>()?;

        StorageMap::with_entries(entries)
            .map_err(|err| AccountComponentTemplateError::StorageMapHasDuplicateKeys(Box::new(err)))
    }

    /// Validates map keys by checking for duplicates.
    ///
    /// Because keys can be represented in a variety of ways, the `to_string()` implementation is
    /// used to check for duplicates.  
    pub(crate) fn validate(&self) -> Result<(), AccountComponentTemplateError> {
        let mut seen_keys = BTreeSet::new();
        for entry in self.entries() {
            // Until instantiating the component we can only tell if keys are duplicate for any
            // predefined entries, so try to build any keys in the map to see if we have
            // duplicates
            entry.key().validate()?;
            entry.value().validate()?;
            if let Ok(key) = entry
                .key()
                .try_build_word(&InitStorageData::default(), StorageValueName::default())
            {
                let key: Digest = key.into();
                if !seen_keys.insert(key) {
                    return Err(AccountComponentTemplateError::StorageMapHasDuplicateKeys(
                        Box::from(format!("key `{key}` is duplicated")),
                    ));
                }
            };
        }
        Ok(())
    }
}

impl From<MapRepresentation> for Vec<MapEntry> {
    fn from(value: MapRepresentation) -> Self {
        value.entries
    }
}

impl Serializable for MapRepresentation {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.entries.write_into(target);
        self.name.write_into(target);
        self.description.write_into(target);
    }
}

impl Deserializable for MapRepresentation {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let entries = Vec::<MapEntry>::read_from(source)?;
        let name = StorageValueName::read_from(source)?;
        let description = Option::<String>::read_from(source)?;
        Ok(Self { entries, name, description })
    }
}
