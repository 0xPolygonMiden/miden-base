use super::Library;
use std::path::PathBuf;
use vm_core::{crypto::hash::Rpo256 as Hasher, Felt, StackInputs, Word, ONE, ZERO};
use vm_processor::{AdviceProvider, DefaultHost, MemAdviceProvider, Process, ProcessState};

mod test_account;
mod test_asset;
mod test_asset_vault;
mod test_epilogue;
mod test_faucet;
mod test_note;
mod test_note_scripts;
mod test_note_setup;
mod test_prologue;
mod test_tx;

// CONSTANTS
// ================================================================================================

const TX_KERNEL_DIR: &str = "sat/internal";

// TESTS
// ================================================================================================

#[test]
fn test_compile() {
    let path = "miden::sat::internal::layout::get_consumed_note_ptr";
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

// HELPER FUNCTIONS
// ================================================================================================

fn build_module_path(dir: &str, file: &str) -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "asm", dir, file].iter().collect()
}
