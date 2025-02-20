use alloc::{collections::BTreeMap, string::String};

use super::StorageValueName;

/// Represents the data required to initialize storage entries when instantiating an
/// [AccountComponent](crate::account::AccountComponent) from a
/// [template](crate::account::AccountComponentTemplate).
///
/// An [`InitStorageData`] can be created from a TOML string when the `std` feature flag is set.
#[derive(Clone, Debug, Default)]
pub struct InitStorageData {
    /// A mapping of storage placeholder names to their corresponding storage values.
    storage_placeholders: BTreeMap<StorageValueName, String>,
}

impl InitStorageData {
    /// Creates a new instance of [InitStorageData].
    /// A [`BTreeMap`] is constructed from the passed iterator, so duplicate keys will cause
    /// overridden values.
    ///
    /// # Parameters
    ///
    /// - `entries`: An iterable collection of key-value pairs.
    pub fn new(entries: impl IntoIterator<Item = (StorageValueName, String)>) -> Self {
        InitStorageData {
            storage_placeholders: entries
                .into_iter()
                .filter(|(k, _)| !k.as_str().is_empty())
                .collect(),
        }
    }

    /// Retrieves a reference to the storage placeholders.
    pub fn placeholders(&self) -> &BTreeMap<StorageValueName, String> {
        &self.storage_placeholders
    }

    /// Returns a reference to the name corresponding to the placeholder, or
    /// [`Option::None`] if the placeholder is not present.
    pub fn get(&self, key: &StorageValueName) -> Option<&String> {
        self.storage_placeholders.get(key)
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;
    use core::error::Error;

    use super::*;
    use crate::account::component::toml::InitStorageDataError;

    #[test]
    fn from_toml_str_with_nested_table_and_flattened() {
        let toml_table = r#"
            [token_metadata]
            max_supply = "1000000000"
            symbol = "ETH"
            decimals = "9"
        "#;

        let toml_inline = r#"
            token_metadata.max_supply = "1000000000"
            token_metadata.symbol = "ETH"
            token_metadata.decimals = "9"
        "#;

        let storage_table = InitStorageData::from_toml(toml_table).unwrap();
        let storage_inline = InitStorageData::from_toml(toml_inline).unwrap();

        assert_eq!(storage_table.placeholders(), storage_inline.placeholders());
    }

    #[test]
    fn from_toml_str_with_deeply_nested_tables() {
        let toml_str = r#"
            [a]
            b = "0xb"

            [a.c]
            d = "0xd"

            [x.y.z]
            w = 42 # NOTE: This gets parsed as string
        "#;

        let storage = InitStorageData::from_toml(toml_str).expect("Failed to parse TOML");
        let key1 = StorageValueName::new("a.b".to_string()).unwrap();
        let key2 = StorageValueName::new("a.c.d".to_string()).unwrap();
        let key3 = StorageValueName::new("x.y.z.w".to_string()).unwrap();

        assert_eq!(storage.get(&key1).unwrap(), "0xb");
        assert_eq!(storage.get(&key2).unwrap(), "0xd");
        assert_eq!(storage.get(&key3).unwrap(), "42");
    }

    #[test]
    fn test_error_on_array() {
        let toml_str = r#"
            token_metadata.v = [1, 2, 3]
        "#;

        let result = InitStorageData::from_toml(toml_str);
        assert_matches::assert_matches!(
            result.unwrap_err(),
            InitStorageDataError::ArraysNotSupported
        );
    }

    #[test]
    fn error_on_empty_subtable() {
        let toml_str = r#"
            [a]
            b = {}
        "#;

        let result = InitStorageData::from_toml(toml_str);
        assert_matches::assert_matches!(result.unwrap_err(), InitStorageDataError::EmptyTable(_));
    }

    #[test]
    fn error_on_duplicate_keys() {
        let toml_str = r#"
            token_metadata.max_supply = "1000000000"
            token_metadata.max_supply = "500000000"
        "#;

        let result = InitStorageData::from_toml(toml_str).unwrap_err();
        // TOML does not support duplicate keys
        assert_matches::assert_matches!(result, InitStorageDataError::InvalidToml(_));
        assert!(result.source().unwrap().to_string().contains("duplicate"));
    }
}
