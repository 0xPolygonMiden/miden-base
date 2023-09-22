#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

use assembly::{utils::Deserializable, Library, LibraryNamespace, MaslLibrary, Version};

#[cfg(test)]
mod tests;

#[cfg(any(test, feature = "testing"))]
pub mod common;

pub mod assembler;
pub mod memory;
pub mod notes;
pub mod transaction;

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

// TEST
// ================================================================================================

#[cfg(feature = "testing")]
mod testing {
    pub use crate::assembler::assembler;
    pub use miden_objects::{
        accounts::Account, notes::Note, transaction::PreparedTransaction, BlockHeader, ChainMmr,
    };
    use std::{env, fs::File, io::Read, path::Path};
    pub use vm_processor::Word;

    /// Loads the specified file and append `code` into its end.
    pub fn load_file_with_code(imports: &str, code: &str, dir: &str, file: &str) -> String {
        let assembly_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("asm").join(dir).join(file);

        let mut module = String::new();
        File::open(assembly_file).unwrap().read_to_string(&mut module).unwrap();
        let complete_code = format!("{imports}{module}{code}");

        // This hack is going around issue #686 on miden-vm
        complete_code.replace("export", "proc")
    }

    pub fn prepare_transaction(
        account: Account,
        account_seed: Option<Word>,
        block_header: BlockHeader,
        chain: ChainMmr,
        notes: Vec<Note>,
        code: &str,
        imports: &str,
        dir: Option<&str>,
        file: Option<&str>,
    ) -> PreparedTransaction {
        let assembler = assembler();

        let code = match (dir, file) {
            (Some(dir), Some(file)) => load_file_with_code(imports, code, dir, file),
            (None, None) => format!("{imports}{code}"),
            _ => panic!("both dir and file must be specified"),
        };

        let program = assembler.compile(code).unwrap();

        PreparedTransaction::new(account, account_seed, block_header, chain, notes, None, program)
            .unwrap()
    }
}

#[cfg(not(feature = "testing"))]
mod testing {}

pub use testing::*;

#[test]
fn test_compile() {
    let path = "miden::sat::internal::layout::get_consumed_note_ptr";
    let miden = MidenLib::default();
    let exists = miden.modules().any(|module| {
        module
            .ast
            .procs()
            .iter()
            .any(|proc| module.path.append(&proc.name).unwrap().as_str() == path)
    });

    assert!(exists);
}
