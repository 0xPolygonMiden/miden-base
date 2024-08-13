#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

use miden_objects::{
    assembly::{mast::MastForest, Library},
    utils::serde::Deserializable,
};

mod auth;
pub use auth::AuthScheme;

pub mod accounts;
pub mod notes;
pub mod transaction;

// RE-EXPORTS
// ================================================================================================

pub use miden_objects::utils;
pub use miden_stdlib::StdLibrary;

// MIDEN LIBRARY
// ================================================================================================

pub struct MidenLib(Library);

impl AsRef<Library> for MidenLib {
    fn as_ref(&self) -> &Library {
        &self.0
    }
}

impl From<MidenLib> for Library {
    fn from(value: MidenLib) -> Self {
        value.0
    }
}

impl From<MidenLib> for MastForest {
    fn from(value: MidenLib) -> Self {
        value.0.into()
    }
}

impl Default for MidenLib {
    fn default() -> Self {
        let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/miden.masl"));
        let contents = Library::read_from_bytes(bytes).expect("failed to read masl!");
        Self(contents)
    }
}

// TESTS
// ================================================================================================

// NOTE: Most kernel-related tests can be found under /miden-tx/kernel_tests
#[cfg(all(test, feature = "std"))]
mod tests {
    use miden_objects::assembly::LibraryPath;

    use super::MidenLib;

    #[test]
    fn test_compile() {
        let path = "miden::account::get_id".parse::<LibraryPath>().unwrap();
        let stdlib = MidenLib::default();
        let exists = stdlib.0.module_infos().any(|module| {
            module
                .procedures()
                .any(|(_, proc)| module.path().clone().append(&proc.name).unwrap() == path)
        });

        assert!(exists);
    }
}
