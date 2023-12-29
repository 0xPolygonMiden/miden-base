use assembly::{ast::ProgramAst, utils::DeserializationError, Assembler};
use miden_stdlib::StdLibrary;

use super::MidenLib;

pub mod memory;

mod inputs;
pub use inputs::ToTransactionKernelInputs;

mod outputs;
pub use outputs::{
    extract_account_storage_delta, notes_try_from_elements, parse_final_account_stub,
    FINAL_ACCOUNT_HASH_WORD_IDX, OUTPUT_NOTES_COMMITMENT_WORD_IDX, TX_SCRIPT_ROOT_WORD_IDX,
};

// TRANSACTION KERNEL
// ================================================================================================

pub struct TransactionKernel;

impl TransactionKernel {
    // KERNEL SOURCE CODE
    // --------------------------------------------------------------------------------------------

    /// Returns MASM source code which encodes the transaction kernel system procedures.
    pub fn kernel() -> &'static str {
        include_str!("../../asm/miden/sat/kernel.masm")
    }

    /// Returns an AST of the transaction kernel executable program.
    ///
    /// # Errors
    /// Returns an error if deserialization of the binary fails.
    pub fn main() -> Result<ProgramAst, DeserializationError> {
        let kernel_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/transaction.masb"));
        ProgramAst::from_bytes(kernel_bytes)
    }

    // ASSEMBLER CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new Miden assembler instantiated with the transaction kernel and loaded with the
    /// Miden stdlib as well as with midenlib.
    pub fn assembler() -> Assembler {
        Assembler::default()
            .with_library(&MidenLib::default())
            .expect("failed to load miden-lib")
            .with_library(&StdLibrary::default())
            .expect("failed to load std-lib")
            .with_kernel(Self::kernel())
            .expect("kernel is well formed")
    }
}
