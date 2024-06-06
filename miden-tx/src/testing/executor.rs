use std::{io::Read, path::PathBuf};

use alloc::{rc::Rc, string::String, vec::Vec};

use miden_lib::transaction::{ToTransactionKernelInputs, TransactionKernel};
use miden_objects::{
    accounts::{AccountCode, AccountId},
    assembly::ProgramAst,
    notes::{NoteId, NoteScript},
    transaction::{
        ExecutedTransaction, PreparedTransaction, TransactionArgs, TransactionInputs,
        TransactionScript,
    },
    vm::{Program, StackOutputs},
    Felt, Word, ZERO,
};
use vm_processor::{
    AdviceInputs, AdviceProvider, DefaultHost, Digest, ExecutionError, ExecutionOptions, Host, Process, RecAdviceProvider, StackInputs
};

use super::MockHost;
use crate::{
    auth::TransactionAuthenticator, DataStore, ScriptTarget, TransactionCompiler,
    TransactionExecutorError, TransactionHost,
};

// MOCK TRANSACTION EXECUTOR
// ================================================================================================

pub struct MockExecutor<H> {
    host: H,
    stack_inputs: Option<StackInputs>,
    advice_inputs: AdviceInputs,
}

impl<H: Host> MockExecutor<H> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    pub fn new(host: H) -> Self {
        Self { host, stack_inputs: None, advice_inputs: AdviceInputs::default() }
    }

    pub fn extend_advice_inputs(mut self, advice_inputs: AdviceInputs) -> Self {
        self.advice_inputs.extend(advice_inputs);
        self
    }

    pub fn stack_inputs(mut self, stack_inputs: StackInputs) -> Self {
        self.stack_inputs = Some(stack_inputs);
        self
    }

    pub fn run_code(&self, imports: &str, code: &str, file_path: Option<PathBuf>) {
        let assembler = TransactionKernel::assembler();
        let code = match file_path {
            Some(file_path) => load_file_with_code(imports, code, file_path),
            None => format!("{imports}{code}"),
        };
    
        let program = assembler.compile(code).unwrap();
        let mut process =
            Process::new(program.kernel().clone(), self.stack_inputs, host, ExecutionOptions::default());
        process.execute(&program)?;
        Ok(process)
    }
}

impl<A> MockExecutor<DefaultHost<A>>
where
    A: AdviceProvider,
{
    pub fn new_with_kernel(adv_provider: A) -> Self {
        let host = DefaultHost::new(adv_provider);
        MockExecutor::new(host)
    }
}

/// Loads the specified file and append `code` into its end.
fn load_file_with_code(imports: &str, code: &str, assembly_file: PathBuf) -> String {
    use std::fs::File;
    use alloc::string::String;

    let mut module = String::new();
    File::open(assembly_file).unwrap().read_to_string(&mut module).unwrap();
    let complete_code = format!("{imports}{module}{code}");

    // This hack is going around issue #686 on miden-vm
    complete_code.replace("export", "proc")
}

