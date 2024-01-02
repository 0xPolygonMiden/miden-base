use std::path::PathBuf;

use miden_objects::{
    transaction::PreparedTransaction,
    vm::{Program, StackInputs},
    Felt, Hasher, Word, ONE, ZERO,
};
use vm_processor::{
    AdviceProvider, ContextId, DefaultHost, MemAdviceProvider, Process, ProcessState,
};

use super::{transaction::ToTransactionKernelInputs, Library};

mod test_account;
mod test_asset;
mod test_asset_vault;
mod test_epilogue;
mod test_faucet;
mod test_note;
mod test_prologue;
mod test_tx;

// CONSTANTS
// ================================================================================================

const TX_KERNEL_DIR: &str = "miden/sat/internal";

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

fn build_tx_inputs(tx: &PreparedTransaction) -> (Program, StackInputs, MemAdviceProvider) {
    let (stack_inputs, advice_inputs) = tx.get_kernel_inputs();
    let advice_provider = MemAdviceProvider::from(advice_inputs);
    (tx.program().clone(), stack_inputs, advice_provider)
}

fn build_module_path(dir: &str, file: &str) -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "asm", dir, file].iter().collect()
}
