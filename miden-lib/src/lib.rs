#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

use miden_objects::{
    assembly::{Library, LibraryNamespace, MaslLibrary, Version},
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

// STANDARD LIBRARY
// ================================================================================================

pub struct MidenLib {
    contents: MaslLibrary,
}

impl Default for MidenLib {
    fn default() -> Self {
        let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/miden.masl"));
        let contents = MaslLibrary::read_from_bytes(bytes).expect("failed to read masl!");
        Self { contents }
    }
}

impl Library for MidenLib {
    type ModuleIterator<'a> = <MaslLibrary as Library>::ModuleIterator<'a>;

    fn root_ns(&self) -> &LibraryNamespace {
        self.contents.root_ns()
    }

    fn version(&self) -> &Version {
        self.contents.version()
    }

    fn modules(&self) -> Self::ModuleIterator<'_> {
        self.contents.modules()
    }

    fn dependencies(&self) -> &[LibraryNamespace] {
        self.contents.dependencies()
    }
}

// TESTS
// ================================================================================================

// NOTE: Most kernel-related tests can be found under /miden-tx/kernel_tests
#[cfg(all(test, feature = "std"))]
mod tests {
    use miden_objects::assembly::Library;

    #[test]
    fn test_compile() {
        let path = "miden::kernels::tx::memory::get_input_note_ptr";
        let miden = super::MidenLib::default();
        let exists = miden.modules().any(|module| {
            module
                .ast
                .procs()
                .iter()
                .any(|proc| module.path.append(&proc.name).unwrap().as_str() == path)
        });

        assert!(exists);
    }
}
