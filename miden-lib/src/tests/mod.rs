use std::path::PathBuf;

use miden_objects::{
    assembly::{AssemblyContext, LibraryPath, ModuleAst},
    vm::StackInputs,
    Felt, Hasher, Word, ONE, ZERO,
};
use vm_processor::{ContextId, MemAdviceProvider, Process, ProcessState};

use super::Library;
use crate::transaction::TransactionKernel;

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

const TX_KERNEL_DIR: &str = "miden/kernels/tx";

// TESTS
// ================================================================================================

#[test]
fn test_compile() {
    let path = "miden::kernels::tx::memory::get_consumed_note_ptr";
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

#[ignore]
#[test]
fn experiment() {
    let assembler = TransactionKernel::assembler();
    let mut context = AssemblyContext::for_module(false);

    let src = "use.miden::account
    use.miden::tx
    use.miden::contracts::wallets::basic->wallet

    # acct proc 5
    export.create_note
        # apply padding
        repeat.8
            push.0 movdn.9
        end

        # create note
        exec.tx::create_note
        # => [ptr, 0, 0, 0, 0, 0, 0, 0, 0]
    end";

    let module = ModuleAst::parse(src).unwrap();
    let path = LibraryPath::new("foo::bar").unwrap();
    let x = assembler.compile_module(&module, Some(&path), &mut context).unwrap();
    println!("{}", x[0]);
    assert!(false);
}

// HELPER FUNCTIONS
// ================================================================================================

fn build_module_path(dir: &str, file: &str) -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "asm", dir, file].iter().collect()
}
