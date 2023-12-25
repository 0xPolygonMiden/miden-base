use assembly::Assembler;
use miden_stdlib::StdLibrary;

use crate::{MidenLib, SatKernel};

pub fn assembler() -> Assembler {
    assembly::Assembler::default()
        .with_library(&MidenLib::default())
        .expect("failed to load miden-lib")
        .with_library(&StdLibrary::default())
        .expect("failed to load std-lib")
        .with_kernel(SatKernel::kernel())
        .expect("kernel is well formed")
}
