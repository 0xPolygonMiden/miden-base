use std::path::PathBuf;

use miden_lib::transaction::TransactionKernel;
use miden_objects::assembly::{Assembler, Library, LibraryNamespace};
#[cfg(feature = "std")]
use vm_processor::{
    AdviceInputs, AdviceProvider, DefaultHost, ExecutionError, Host, Process, Program, StackInputs,
};

// MOCK CODE EXECUTOR
// ================================================================================================

/// Helper for executing arbitrary code within arbitrary hosts.
pub struct CodeExecutor<H> {
    host: H,
    stack_inputs: Option<StackInputs>,
    advice_inputs: AdviceInputs,
}

impl<H: Host> CodeExecutor<H> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    pub fn new(host: H) -> Self {
        Self {
            host,
            stack_inputs: None,
            advice_inputs: AdviceInputs::default(),
        }
    }

    pub fn extend_advice_inputs(mut self, advice_inputs: AdviceInputs) -> Self {
        self.advice_inputs.extend(advice_inputs);
        self
    }

    pub fn stack_inputs(mut self, stack_inputs: StackInputs) -> Self {
        self.stack_inputs = Some(stack_inputs);
        self
    }

    /// Compiles and runs the desired code in the host and returns the [Process] state
    pub fn run(self, code: &str, assembler: Assembler) -> Result<Process<H>, ExecutionError> {
        let program = assembler.assemble_program(code).unwrap();
        self.execute_program(program)
    }

    pub fn execute_program(self, program: Program) -> Result<Process<H>, ExecutionError> {
        let mut process = Process::new_debug(
            program.kernel().clone(),
            self.stack_inputs.unwrap_or_default(),
            self.host,
        );
        process.execute(&program)?;

        Ok(process)
    }
}

impl<A> CodeExecutor<DefaultHost<A>>
where
    A: AdviceProvider,
{
    pub fn with_advice_provider(adv_provider: A) -> Self {
        let mut host = DefaultHost::new(adv_provider);
        let workspace_dir = env!("CARGO_MANIFEST_DIR");
        let path = PathBuf::from(format!("{workspace_dir}/../miden-lib/asm/kernels/transaction/"));

        let namespace = "kernel".parse::<LibraryNamespace>().expect("invalid base namespace");
        let test_lib =
            Library::from_dir(path.join("lib"), namespace, TransactionKernel::assembler()).unwrap();
        host.load_mast_forest(test_lib.mast_forest().clone());
        CodeExecutor::new(host)
    }
}
