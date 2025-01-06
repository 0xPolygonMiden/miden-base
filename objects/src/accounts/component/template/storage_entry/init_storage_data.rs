use alloc::{collections::BTreeMap, string::String};

use super::TemplateValue;

/// Represents the data required to initialize storage entries when instantiating an
/// [AccountComponent](crate::accounts::AccountComponent).
#[derive(Clone, Debug, Default)]
pub struct InitStorageData {
    /// A mapping of template key names to their corresponding template values.
    template_values: BTreeMap<String, TemplateValue>,
}

impl InitStorageData {
    /// Creates a new instance of [InitStorageData] with generic iterators.
    ///
    /// # Parameters
    ///
    /// - `template_keys`: An iterable collection of key-value pairs for template keys.
    pub fn new<K>(template_values: K) -> Self
    where
        K: IntoIterator<Item = (String, TemplateValue)>,
    {
        InitStorageData {
            template_values: template_values.into_iter().collect(),
        }
    }

    /// Retrieves a reference to the template values.
    pub fn get_template_keys(&self) -> &BTreeMap<String, TemplateValue> {
        &self.template_values
    }

    /// Returns a reference to the [TemplateValue] corresponding to the key, or [`Option::None`]
    /// if the key is not present.
    pub fn get(&self, key: &str) -> Option<&TemplateValue> {
        self.template_values.get(key)
    }
}
