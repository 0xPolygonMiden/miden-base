use alloc::sync::Arc;

use miden_objects::{
    assembly::{mast::MastForest, Library},
    utils::{serde::Deserializable, sync::LazyLock},
};

// CONSTANTS
// ================================================================================================

const UTILS_LIB_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/utils.masl"));

// UTILITIES LIBRARY
// ================================================================================================

#[derive(Clone)]
pub struct UtilsLib(Library);

impl UtilsLib {
    /// Returns a reference to the [`MastForest`] of the inner [`Library`].
    pub fn mast_forest(&self) -> &Arc<MastForest> {
        self.0.mast_forest()
    }
}

impl AsRef<Library> for UtilsLib {
    fn as_ref(&self) -> &Library {
        &self.0
    }
}

impl From<UtilsLib> for Library {
    fn from(value: UtilsLib) -> Self {
        value.0
    }
}

impl Default for UtilsLib {
    fn default() -> Self {
        static UTILS_LIB: LazyLock<UtilsLib> = LazyLock::new(|| {
            let contents =
                Library::read_from_bytes(UTILS_LIB_BYTES).expect("failed to read miden lib masl!");
            UtilsLib(contents)
        });
        UTILS_LIB.clone()
    }
}
