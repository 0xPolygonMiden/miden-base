use std::path::PathBuf;

use miden_lib::MidenLib;
use miden_objects::assembly::Library;

// CONSTANTS
// ================================================================================================

pub const TX_KERNEL_DIR: &str = "miden/kernels/tx";

// TESTS
// ================================================================================================

#[test]
fn test_compile() {
    let path = "miden::kernels::tx::memory::get_consumed_note_ptr";
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

// HELPER FUNCTIONS
// ================================================================================================

pub fn build_module_path(dir: &str, file: &str) -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "asm", dir, file].iter().collect()
}
