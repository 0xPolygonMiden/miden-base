use std::{fs::File, io::Read, path::PathBuf};

use miden_lib::transaction::{memory, ToTransactionKernelInputs, TransactionKernel};
use miden_objects::{
    notes::NoteAssets,
    transaction::{OutputNotes, PreparedTransaction, TransactionInputs, TransactionScript},
    Felt, StarkField,
};
use mock::host::MockHost;
use vm_processor::{
    AdviceInputs, AdviceProvider, DefaultHost, ExecutionError, ExecutionOptions, Host, Process,
    StackInputs, Word,
};

pub mod builders;
pub mod constants;
pub mod mock;
pub mod procedures;

// TEST BRACE
// ================================================================================================

/// Loads the specified file and append `code` into its end.
fn load_file_with_code(imports: &str, code: &str, assembly_file: PathBuf) -> String {
    let mut module = String::new();
    File::open(assembly_file).unwrap().read_to_string(&mut module).unwrap();
    let complete_code = format!("{imports}{module}{code}");

    // This hack is going around issue #686 on miden-vm
    complete_code.replace("export", "proc")
}

/// Inject `code` along side the specified file and run it
pub fn run_tx(tx: &PreparedTransaction) -> Result<Process<MockHost>, ExecutionError> {
    run_tx_with_inputs(tx, AdviceInputs::default())
}

pub fn run_tx_with_inputs(
    tx: &PreparedTransaction,
    inputs: AdviceInputs,
) -> Result<Process<MockHost>, ExecutionError> {
    let program = tx.program().clone();
    let (stack_inputs, mut advice_inputs) = tx.get_kernel_inputs();
    advice_inputs.extend(inputs);
    let host = MockHost::new(tx.account().into(), advice_inputs);
    let exec_options = ExecutionOptions::default().with_tracing();
    let mut process = Process::new(program.kernel().clone(), stack_inputs, host, exec_options);
    process.execute(&program)?;
    Ok(process)
}

/// Inject `code` along side the specified file and run it
pub fn run_within_tx_kernel<A>(
    imports: &str,
    code: &str,
    stack_inputs: StackInputs,
    mut adv: A,
    file_path: Option<PathBuf>,
) -> Result<Process<DefaultHost<A>>, ExecutionError>
where
    A: AdviceProvider,
{
    // mock account method for testing from root context
    adv.insert_into_map(Word::default(), vec![Felt::new(255)]).unwrap();

    let assembler = TransactionKernel::assembler();

    let code = match file_path {
        Some(file_path) => load_file_with_code(imports, code, file_path),
        None => format!("{imports}{code}"),
    };

    let program = assembler.compile(code).unwrap();

    let host = DefaultHost::new(adv);
    let exec_options = ExecutionOptions::default().with_tracing();
    let mut process = Process::new(program.kernel().clone(), stack_inputs, host, exec_options);
    process.execute(&program)?;
    Ok(process)
}

/// Inject `code` along side the specified file and run it
pub fn run_within_host<H: Host>(
    imports: &str,
    code: &str,
    stack_inputs: StackInputs,
    host: H,
    file_path: Option<PathBuf>,
) -> Result<Process<H>, ExecutionError> {
    let assembler = TransactionKernel::assembler();
    let code = match file_path {
        Some(file_path) => load_file_with_code(imports, code, file_path),
        None => format!("{imports}{code}"),
    };

    let program = assembler.compile(code).unwrap();
    let mut process =
        Process::new(program.kernel().clone(), stack_inputs, host, ExecutionOptions::default());
    process.execute(&program)?;
    Ok(process)
}

// TEST HELPERS
// ================================================================================================
pub fn consumed_note_data_ptr(note_idx: u32) -> memory::MemoryAddress {
    memory::CONSUMED_NOTE_SECTION_OFFSET + (1 + note_idx) * memory::NOTE_MEM_SIZE
}

pub fn prepare_transaction(
    tx_inputs: TransactionInputs,
    tx_script: Option<TransactionScript>,
    code: &str,
    file_path: Option<PathBuf>,
) -> PreparedTransaction {
    let assembler = TransactionKernel::assembler();

    let code = match file_path {
        Some(file_path) => load_file_with_code("", code, file_path),
        None => code.to_string(),
    };

    let program = assembler.compile(code).unwrap();
    PreparedTransaction::new(program, tx_script, tx_inputs)
}
