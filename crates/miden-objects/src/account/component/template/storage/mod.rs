use alloc::{boxed::Box, string::String, vec::Vec};

use vm_core::{
    utils::{ByteReader, ByteWriter, Deserializable, Serializable},
    Felt, FieldElement,
};
use vm_processor::DeserializationError;

mod entry_content;
pub use entry_content::*;

use super::AccountComponentTemplateError;
use crate::account::StorageSlot;

mod placeholder;
pub use placeholder::{
    PlaceholderTypeRequirement, StorageValueName, StorageValueNameError, TemplateTypeError,
};

mod init_storage_data;
pub use init_storage_data::InitStorageData;

#[cfg(feature = "std")]
pub mod toml;

/// Alias used for iterators that collect all placeholders and their types within a component
/// template.
pub type TemplateRequirementsIter<'a> =
    Box<dyn Iterator<Item = (StorageValueName, PlaceholderTypeRequirement)> + 'a>;

// STORAGE ENTRY
// ================================================================================================

/// Represents a single entry in the component's storage layout.
///
/// Each entry can describe:
/// - A value slot with a single word.
/// - A map slot with a key-value map that occupies one storage slot.
/// - A multi-slot entry spanning multiple contiguous slots with multiple words (but not maps) that
///   represent a single logical value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum StorageEntry {
    /// A value slot, which can contain one word.
    Value {
        /// The numeric index of this map slot in the component's storage.
        slot: u8,
        /// An description of a word, representing either a predefined value or a templated one.
        word_entry: WordRepresentation,
    },

    /// A map slot, containing multiple key-value pairs. Keys and values are hex-encoded strings.
    Map {
        /// The numeric index of this map slot in the component's storage.
        slot: u8,
        /// A list of key-value pairs to initialize in this map slot.
        map: MapRepresentation,
    },

    /// A multi-slot entry, representing a single logical value across multiple slots.
    MultiSlot {
        /// The human-readable name of this multi-slot entry.
        name: StorageValueName,
        /// An optional description for the slot, explaining its purpose.
        description: Option<String>,
        /// The indices of the slots that form this multi-slot entry.
        slots: Vec<u8>,
        /// A list of values to fill the logical slot, with a length equal to the amount of slots.
        values: Vec<[FeltRepresentation; 4]>,
    },
}

impl StorageEntry {
    pub fn new_value(slot: u8, word_entry: impl Into<WordRepresentation>) -> Self {
        StorageEntry::Value { slot, word_entry: word_entry.into() }
    }

    pub fn new_map(slot: u8, map: MapRepresentation) -> Self {
        StorageEntry::Map { slot, map }
    }

    pub fn new_multislot(
        name: impl Into<StorageValueName>,
        description: Option<String>,
        slots: Vec<u8>,
        values: Vec<[FeltRepresentation; 4]>,
    ) -> Self {
        StorageEntry::MultiSlot {
            name: name.into(),
            description: description.map(Into::into),
            slots,
            values,
        }
    }

    pub fn name(&self) -> &StorageValueName {
        match self {
            StorageEntry::Value { word_entry, .. } => {
                word_entry.name().expect("by construction, all top level entries have names")
            },
            StorageEntry::Map { map, .. } => map.name(),
            StorageEntry::MultiSlot { name, .. } => name,
        }
    }

    /// Returns the slot indices that the storage entry covers.
    pub fn slot_indices(&self) -> &[u8] {
        match self {
            StorageEntry::MultiSlot { slots, .. } => slots.as_slice(),
            StorageEntry::Value { slot, .. } => core::slice::from_ref(slot),
            StorageEntry::Map { slot, .. } => core::slice::from_ref(slot),
        }
    }

    /// Returns an iterator over all of the storage entries's value names, alongside their
    /// expected type.
    pub fn template_requirements(&self) -> TemplateRequirementsIter {
        let requirements = match self {
            StorageEntry::Value { word_entry, .. } => {
                word_entry.template_requirements(StorageValueName::default())
            },
            StorageEntry::Map { map: map_entries, .. } => map_entries.template_requirements(),
            StorageEntry::MultiSlot { values, .. } => {
                Box::new(values.iter().flat_map(move |word| {
                    word.iter().flat_map(move |f| f.template_requirements(self.name().clone()))
                }))
            },
        };

        requirements
    }

    /// Attempts to convert the storage entry into a list of [StorageSlot].
    ///
    /// - StorageEntry::Value would convert to a [StorageSlot::Value]
    /// - StorageEntry::MultiSlot would convert to as many [StorageSlot::Value] as defined
    /// - StorageEntry::Map would convert to a [StorageSlot::Map]
    ///
    /// Each of the entry's values could be templated. These values are replaced for values found
    /// in `init_storage_data`, identified by its key.
    pub fn try_build_storage_slots(
        &self,
        init_storage_data: &InitStorageData,
    ) -> Result<Vec<StorageSlot>, AccountComponentTemplateError> {
        match self {
            StorageEntry::Value { word_entry, .. } => {
                let slot =
                    word_entry.try_build_word(init_storage_data, StorageValueName::default())?;
                Ok(vec![StorageSlot::Value(slot)])
            },
            StorageEntry::Map { map: values, .. } => {
                let storage_map = values.try_build_map(init_storage_data)?;
                Ok(vec![StorageSlot::Map(storage_map)])
            },
            StorageEntry::MultiSlot { values, .. } => Ok(values
                .iter()
                .map(|word_repr| {
                    let mut result = [Felt::ZERO; 4];

                    for (index, felt_repr) in word_repr.iter().enumerate() {
                        result[index] = felt_repr
                            .clone()
                            .try_build_felt(init_storage_data, self.name().clone())?;
                    }
                    // SAFETY: result is guaranteed to have all its 4 indices rewritten
                    Ok(StorageSlot::Value(result))
                })
                .collect::<Result<Vec<StorageSlot>, _>>()?),
        }
    }

    /// Validates the storage entry for internal consistency.
    pub(super) fn validate(&self) -> Result<(), AccountComponentTemplateError> {
        match self {
            StorageEntry::Map { map, .. } => map.validate(),
            StorageEntry::MultiSlot { slots, values, .. } => {
                if slots.len() != values.len() {
                    return Err(AccountComponentTemplateError::MultiSlotArityMismatch);
                } else {
                    let mut all_slots = slots.clone();
                    all_slots.sort_unstable();
                    for slots in all_slots.windows(2) {
                        if slots[1] == slots[0] {
                            return Err(AccountComponentTemplateError::DuplicateSlot(slots[0]));
                        }

                        if slots[1] != slots[0] + 1 {
                            return Err(AccountComponentTemplateError::NonContiguousSlots(
                                slots[0], slots[1],
                            ));
                        }
                    }
                }
                Ok(())
            },
            StorageEntry::Value { .. } => Ok(()),
        }
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for StorageEntry {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            StorageEntry::Value { slot, word_entry } => {
                target.write_u8(0u8);
                target.write_u8(*slot);
                target.write(word_entry);
            },
            StorageEntry::Map { slot, map, .. } => {
                target.write_u8(1u8);
                target.write_u8(*slot);
                target.write(map);
            },
            StorageEntry::MultiSlot { name, description, slots, values } => {
                target.write_u8(2u8);
                target.write(name);
                target.write(description);
                target.write(slots);
                target.write(values);
            },
        }
    }
}

impl Deserializable for StorageEntry {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let variant_tag = source.read_u8()?;
        match variant_tag {
            0 => {
                let slot = source.read_u8()?;
                let word_entry: WordRepresentation = source.read()?;
                Ok(StorageEntry::Value { slot, word_entry })
            },
            1 => {
                let slot = source.read_u8()?;
                let map: MapRepresentation = source.read()?;
                Ok(StorageEntry::Map { slot, map })
            },
            2 => {
                let name: StorageValueName = source.read()?;
                let description: Option<String> = source.read()?;
                let slots: Vec<u8> = source.read()?;
                let values: Vec<[FeltRepresentation; 4]> = source.read()?;
                Ok(StorageEntry::MultiSlot { name, description, slots, values })
            },
            _ => Err(DeserializationError::InvalidValue(format!(
                "unknown variant tag `{}` for StorageEntry",
                variant_tag
            ))),
        }
    }
}

// MAP ENTRY
// ================================================================================================

/// Key-value entry for storage maps.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct MapEntry {
    key: WordRepresentation,
    value: WordRepresentation,
}

impl MapEntry {
    pub fn new(key: impl Into<WordRepresentation>, value: impl Into<WordRepresentation>) -> Self {
        Self { key: key.into(), value: value.into() }
    }

    pub fn key(&self) -> &WordRepresentation {
        &self.key
    }

    pub fn value(&self) -> &WordRepresentation {
        &self.value
    }

    pub fn into_parts(self) -> (WordRepresentation, WordRepresentation) {
        let MapEntry { key, value } = self;
        (key, value)
    }

    pub fn template_requirements(
        &self,
        placeholder_prefix: StorageValueName,
    ) -> TemplateRequirementsIter<'_> {
        let key_iter = self.key.template_requirements(placeholder_prefix.clone());
        let value_iter = self.value.template_requirements(placeholder_prefix);

        Box::new(key_iter.chain(value_iter))
    }
}

impl Serializable for MapEntry {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.key.write_into(target);
        self.value.write_into(target);
    }
}

impl Deserializable for MapEntry {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let key = WordRepresentation::read_from(source)?;
        let value = WordRepresentation::read_from(source)?;
        Ok(MapEntry { key, value })
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use core::panic;
    use std::string::ToString;

    use assembly::Assembler;
    use semver::Version;
    use vm_core::{
        utils::{Deserializable, Serializable},
        Felt, FieldElement, Word,
    };

    use crate::{
        account::{
            component::template::{
                AccountComponentMetadata, InitStorageData, MapEntry, MapRepresentation,
                StorageValueName,
            },
            AccountComponent, AccountComponentTemplate, AccountType, FeltRepresentation,
            StorageEntry, StorageSlot, TemplateTypeError, WordRepresentation,
        },
        digest,
        errors::AccountComponentTemplateError,
        testing::account_code::CODE,
        AccountError,
    };

    #[test]
    fn test_storage_entry_serialization() {
        let felt_array: [FeltRepresentation; 4] = [
            FeltRepresentation::from(Felt::new(0xabc)),
            FeltRepresentation::from(Felt::new(1218)),
            FeltRepresentation::from(Felt::new(0xdba3)),
            FeltRepresentation::new_template(
                "felt",
                StorageValueName::new("slot3").unwrap(),
                Some("dummy description".into()),
            ),
        ];

        let test_word: Word = digest!("0x000001").into();
        let test_word = test_word.map(FeltRepresentation::from);

        let map_representation = MapRepresentation::new(
            vec![
                MapEntry {
                    key: WordRepresentation::new_template(
                        "word",
                        StorageValueName::new("foo").unwrap(),
                        None,
                    ),
                    value: WordRepresentation::new_value(test_word.clone(), None, None),
                },
                MapEntry {
                    key: WordRepresentation::new_value(test_word.clone(), None, None),
                    value: WordRepresentation::new_template(
                        "word",
                        StorageValueName::new("bar").unwrap(),
                        Some("bar description".into()),
                    ),
                },
                MapEntry {
                    key: WordRepresentation::new_template(
                        "word",
                        StorageValueName::new("baz").unwrap(),
                        Some("baz description".into()),
                    ),
                    value: WordRepresentation::new_value(test_word, None, None),
                },
            ],
            StorageValueName::new("map").unwrap(),
            Some("A storage map entry".into()),
        );

        let storage = vec![
            StorageEntry::new_value(0, felt_array.clone()),
            StorageEntry::new_map(1, map_representation),
            StorageEntry::new_multislot(
                StorageValueName::new("multi").unwrap(),
                Some("Multi slot entry".into()),
                vec![2, 3],
                vec![
                    [
                        FeltRepresentation::new_template(
                            "felt",
                            StorageValueName::new("test").unwrap(),
                            None,
                        ),
                        FeltRepresentation::new_template(
                            "felt",
                            StorageValueName::new("test2").unwrap(),
                            None,
                        ),
                        FeltRepresentation::new_template(
                            "felt",
                            StorageValueName::new("test3").unwrap(),
                            None,
                        ),
                        FeltRepresentation::new_template(
                            "felt",
                            StorageValueName::new("test4").unwrap(),
                            None,
                        ),
                    ],
                    felt_array,
                ],
            ),
            StorageEntry::new_value(
                4,
                WordRepresentation::new_template(
                    "word",
                    StorageValueName::new("single").unwrap(),
                    None,
                ),
            ),
        ];

        let config = AccountComponentMetadata {
            name: "Test Component".into(),
            description: "This is a test component".into(),
            version: Version::parse("1.0.0").unwrap(),
            supported_types: std::collections::BTreeSet::from([AccountType::FungibleFaucet]),
            storage,
        };
        let toml = config.as_toml().unwrap();
        let deserialized = AccountComponentMetadata::from_toml(&toml).unwrap();

        assert_eq!(deserialized, config);
    }

    #[test]
    pub fn test_toml() {
        let toml_text = r#"
        name = "Test Component"
        description = "This is a test component"
        version = "1.0.1"
        supported-types = ["FungibleFaucet", "RegularAccountImmutableCode"]

        [[storage]]
        name = "map_entry"
        slot = 0
        values = [
            { key = "0x1", value = ["0x1","0x2","0x3","0"]},
            { key = "0x3", value = "0x123" }, 
            { key = { name = "map_key_template", description = "this tests that the default type is correctly set" }, value = "0x3" }
        ]

        [[storage]]
        name = "token_metadata"
        description = "Contains metadata about the token associated to the faucet account"
        slot = 1
        value = [
            { type = "felt", name = "max_supply", description = "Maximum supply of the token in base units" }, # placeholder
            { type = "tokensymbol", value = "TST" }, # hardcoded non-felt type
            { type = "u8", name = "decimals", description = "Number of decimal places" }, # placeholder
            { value = "0" }, 
        ]

        [[storage]]
        name = "default_recallable_height"
        slot = 2
        type = "u32"
        "#;

        let component_metadata = AccountComponentMetadata::from_toml(toml_text).unwrap();
        let requirements = component_metadata.get_placeholder_requirements();

        assert_eq!(requirements.len(), 4);

        let supply = requirements
            .get(&StorageValueName::new("token_metadata.max_supply").unwrap())
            .unwrap();
        assert_eq!(supply.r#type.to_string(), "felt");

        let decimals = requirements
            .get(&StorageValueName::new("token_metadata.decimals").unwrap())
            .unwrap();
        assert_eq!(decimals.r#type.to_string(), "u8");

        let default_recallable_height = requirements
            .get(&StorageValueName::new("default_recallable_height").unwrap())
            .unwrap();
        assert_eq!(default_recallable_height.r#type.to_string(), "u32");

        let map_key_template = requirements
            .get(&StorageValueName::new("map_entry.map_key_template").unwrap())
            .unwrap();
        assert_eq!(map_key_template.r#type.to_string(), "word");

        let library = Assembler::default().assemble_library([CODE]).unwrap();
        let template = AccountComponentTemplate::new(component_metadata, library);

        let template_bytes = template.to_bytes();
        let template_deserialized =
            AccountComponentTemplate::read_from_bytes(&template_bytes).unwrap();
        assert_eq!(template, template_deserialized);

        // Fail to parse because 2800 > u8
        let storage_placeholders = InitStorageData::new([
            (
                StorageValueName::new("map_entry.map_key_template").unwrap(),
                "0x123".to_string(),
            ),
            (
                StorageValueName::new("token_metadata.max_supply").unwrap(),
                20_000u64.to_string(),
            ),
            (StorageValueName::new("token_metadata.decimals").unwrap(), "2800".into()),
            (StorageValueName::new("default_recallable_height").unwrap(), "0".into()),
        ]);

        let component = AccountComponent::from_template(&template, &storage_placeholders);
        assert_matches::assert_matches!(
            component,
            Err(AccountError::AccountComponentTemplateInstantiationError(
                AccountComponentTemplateError::StorageValueParsingError(
                    TemplateTypeError::ParseError(_, _)
                )
            ))
        );

        // Instantiate succesfully

        let storage_placeholders = InitStorageData::new([
            (
                StorageValueName::new("map_entry.map_key_template").unwrap(),
                "0x123".to_string(),
            ),
            (
                StorageValueName::new("token_metadata.max_supply").unwrap(),
                20_000u64.to_string(),
            ),
            (StorageValueName::new("token_metadata.decimals").unwrap(), "128".into()),
            (StorageValueName::new("default_recallable_height").unwrap(), "0".into()),
        ]);

        let component = AccountComponent::from_template(&template, &storage_placeholders).unwrap();
        assert_eq!(
            component.supported_types(),
            &[AccountType::FungibleFaucet, AccountType::RegularAccountImmutableCode]
                .into_iter()
                .collect()
        );

        let storage_map = component.storage_slots.first().unwrap();
        match storage_map {
            StorageSlot::Map(storage_map) => assert_eq!(storage_map.entries().count(), 3),
            _ => panic!("should be map"),
        }

        let value_entry = component.storage_slots().get(2).unwrap();
        match value_entry {
            StorageSlot::Value(v) => {
                assert_eq!(v, &[Felt::ZERO, Felt::ZERO, Felt::ZERO, Felt::ZERO])
            },
            _ => panic!("should be value"),
        }

        let failed_instantiation =
            AccountComponent::from_template(&template, &InitStorageData::default());

        assert_matches::assert_matches!(
            failed_instantiation,
            Err(AccountError::AccountComponentTemplateInstantiationError(
                AccountComponentTemplateError::PlaceholderValueNotProvided(_)
            ))
        );
    }
}
