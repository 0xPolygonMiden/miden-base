use assembly::Assembler;
use miden_lib::{MidenLib, SatKernel};
use miden_stdlib::StdLibrary;

mod account;
mod block;
mod chain;
mod constants;
mod notes;
mod transaction;
mod utils;

// RE-EXPORTS
// ================================================================================================
pub use account::*;
pub use block::*;
pub use chain::*;
pub use constants::*;
pub use notes::*;
pub use transaction::*;
pub use utils::*;

// ASSEMBLER
// ================================================================================================
pub fn assembler() -> Assembler {
    assembly::Assembler::default()
        .with_library(&MidenLib::default())
        .expect("failed to load miden-lib")
        .with_library(&StdLibrary::default())
        .expect("failed to load std-lib")
        .with_kernel(SatKernel::kernel())
        .expect("kernel is well formed")
}
