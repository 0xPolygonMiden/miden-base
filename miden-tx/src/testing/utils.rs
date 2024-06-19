#[cfg(feature = "std")]
use std::{fs::File, io::Read, path::PathBuf, string::String};

#[cfg(not(target_family = "wasm"))]
use miden_lib::transaction::TransactionKernel;
use miden_lib::transaction::{memory, ToTransactionKernelInputs};
use miden_objects::transaction::PreparedTransaction;
#[cfg(feature = "std")]
use miden_objects::transaction::{TransactionArgs, TransactionInputs};
#[cfg(not(target_family = "wasm"))]
use vm_processor::{AdviceInputs, ExecutionError, Process};
#[cfg(feature = "std")]
use vm_processor::{ExecutionOptions, Host, StackInputs};

use crate::testing::MockHost;

// TEST BRACE
// ================================================================================================

/// Loads the specified file and append `code` into its end.
#[cfg(feature = "std")]
pub fn load_file_with_code(imports: &str, code: &str, assembly_file: PathBuf) -> String {
    let mut module = String::new();
    File::open(assembly_file).unwrap().read_to_string(&mut module).unwrap();
    let complete_code = format!("{imports}{module}{code}");

    // This hack is going around issue #686 on miden-vm
    complete_code.replace("export", "proc")
}

/// Inject `code` along side the specified file and run it
pub fn run_tx_with_inputs(
    tx: &PreparedTransaction,
    inputs: AdviceInputs,
) -> Result<Process<MockHost>, ExecutionError> {
    let program = tx.program().clone();
    let (stack_inputs, mut advice_inputs) = tx.get_kernel_inputs();
    advice_inputs.extend(inputs);
    let host = MockHost::new(tx.account().into(), advice_inputs);
    let mut process = Process::new_debug(program.kernel().clone(), stack_inputs, host);
    process.execute(&program)?;
    Ok(process)
}

#[cfg(feature = "std")]
pub fn run_within_host<H: Host>(
    code: &str,
    stack_inputs: StackInputs,
    host: H,
) -> Result<Process<H>, ExecutionError> {
    let assembler = TransactionKernel::assembler();
    let program = assembler.compile(code).unwrap();
    let mut process = Process::new(
        program.kernel().clone(),
        stack_inputs,
        host,
        ExecutionOptions::default().with_tracing(),
    );
    process.execute(&program)?;
    Ok(process)
}

// TEST HELPERS
// ================================================================================================
pub fn consumed_note_data_ptr(note_idx: u32) -> memory::MemoryAddress {
    memory::CONSUMED_NOTE_DATA_SECTION_OFFSET + note_idx * memory::NOTE_MEM_SIZE
}

#[cfg(feature = "std")]
pub fn prepare_transaction(
    tx_inputs: TransactionInputs,
    tx_args: TransactionArgs,
    code: &str,
) -> PreparedTransaction {
    let assembler = TransactionKernel::assembler().with_debug_mode(true);
    let program = assembler.compile(code).unwrap();
    PreparedTransaction::new(program, tx_inputs, tx_args)
}
