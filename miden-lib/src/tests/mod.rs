/// NOTE: Most kernel-related tests can be found under /miden-tx/kernel_tests
// CONSTANTS
// ================================================================================================
use miden_objects::assembly::Library;

// TESTS
// ================================================================================================

#[test]
fn test_compile() {
    let path = "miden::kernels::tx::memory::get_consumed_note_ptr";
    let var_name = super::MidenLib::default();
    let miden = var_name;
    let exists = miden.modules().any(|module| {
        module
            .ast
            .procs()
            .iter()
            .any(|proc| module.path.append(&proc.name).unwrap().as_str() == path)
    });

    assert!(exists);
}
