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
use vm_processor::{Host, StackInputs};

use crate::testing::MockHost;

// TEST HELPERS
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

/// Runs `code` under the Miden VM.
///
/// The `code` is compiled and linked against the miden library. The VM's stack is initialized to
/// `stack_inputs`, and the communication host is set to `host`. The `host` is used to handle events
/// and provide the advice inputs.
///
/// # Returns
///
/// An error if a failure occurred or the process after the execution finishes.
#[cfg(feature = "std")]
pub fn run_within_host<H: Host>(
    code: &str,
    stack_inputs: StackInputs,
    host: H,
) -> Result<Process<H>, ExecutionError> {
    let assembler = TransactionKernel::assembler().with_debug_mode(true);
    let program = assembler.compile(code).unwrap();
    let mut process = Process::new_debug(program.kernel().clone(), stack_inputs, host);
    process.execute(&program)?;
    Ok(process)
}

pub fn consumed_note_data_ptr(note_idx: u32) -> memory::MemoryAddress {
    memory::CONSUMED_NOTE_DATA_SECTION_OFFSET + note_idx * memory::NOTE_MEM_SIZE
}

/// Constructs a [PreparedTransaction] which can be later executed.
///
/// Note: To execute the prepared transaction see [run_tx_with_inputs].
///
/// # Returns
///
/// A [PreparedTransaction] object representing a transaction to be executed.
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

/// Executes a [PreparedTransaction] under the Miden VM with a host initialized with [AdviceInputs].
///
/// Note: To construct a prepared transaction see [prepare_transaction].
///
/// # Returns
///
/// An error if a failure occurred or the process after the execution finishes.
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
