use crate::{MidenLib, SatKernel};
use assembly::Assembler;
use miden_stdlib::StdLibrary;

pub fn assembler() -> Assembler {
    assembly::Assembler::default()
        .with_library(&MidenLib::default())
        .expect("failed to load miden-lib")
        .with_library(&StdLibrary::default())
        .expect("failed to load std-lib")
        .with_kernel(SatKernel::kernel_module_as_str())
        .expect("kernel is well formed")
}
