#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

use assembly::{utils::Deserializable, Library, LibraryNamespace, MaslLibrary, Version};

#[cfg(test)]
mod tests;

mod auth;
pub use auth::AuthScheme;

pub mod assembler;
pub mod faucets;
pub mod memory;
pub mod notes;
pub mod outputs;
pub mod transaction;
pub mod wallets;

// STANDARD LIBRARY
// ================================================================================================

pub struct MidenLib {
    contents: MaslLibrary,
}

impl Default for MidenLib {
    fn default() -> Self {
        let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/miden.masl"));
        let contents = MaslLibrary::read_from_bytes(bytes).expect("failed to read std masl!");
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

// SINGLE ACCOUNT TRANSACTION (SAT) KERNEL
// ================================================================================================

pub struct SatKernel;

impl SatKernel {
    // SAT KERNEL METHODS
    // --------------------------------------------------------------------------------------------
    /// Returns masm source code which encodes the transaction kernel system procedures.
    pub fn kernel() -> &'static str {
        include_str!("../asm/miden/sat/kernel.masm")
    }

    // SAT KERNEL MAIN
    // --------------------------------------------------------------------------------------------
    /// Returns masm source code which encodes the transaction kernel main procedure.
    pub fn main() -> &'static str {
        "\
        use.miden::sat::internal::main

        begin
            exec.main::main
        end
        "
    }
}
