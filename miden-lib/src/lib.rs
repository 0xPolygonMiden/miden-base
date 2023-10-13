#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

use assembly::{utils::Deserializable, Library, LibraryNamespace, MaslLibrary, Version};

#[cfg(test)]
mod tests;

pub mod assembler;
pub mod memory;
pub mod notes;
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
        include_str!("../asm/sat/kernel.masm")
    }

    // SAT KERNEL SECTIONS
    // --------------------------------------------------------------------------------------------
    /// Returns masm source code which encodes the transaction kernel prologue.
    pub fn prologue() -> &'static str {
        "\
        use.miden::sat::internal::prologue

        begin
            exec.prologue::prepare_transaction
        end
        "
    }

    /// Returns masm source code which encodes the transaction kernel epilogue.
    pub fn epilogue() -> &'static str {
        "\
        use.miden::sat::internal::epilogue

        begin
            exec.epilogue::finalize_transaction
        end"
    }

    /// Returns masm source code which encodes the transaction kernel note setup script.
    pub fn note_setup() -> &'static str {
        "\
        use.miden::sat::internal::note_setup

        begin
            exec.note_setup::prepare_note
        end
        "
    }

    /// Returns masm source code which encodes the transaction kernel note teardown script.
    pub fn note_processing_teardown() -> &'static str {
        "\
        use.miden::sat::internal::note

        begin
            exec.note::reset_current_consumed_note_ptr
        end
        "
    }
}
