use alloc::string::String;
use std::{io::Read, path::PathBuf};

#[cfg(not(target_family = "wasm"))]
use miden_lib::transaction::TransactionKernel;
#[cfg(feature = "std")]
use vm_processor::{
    AdviceInputs, AdviceProvider, DefaultHost, ExecutionError, ExecutionOptions, Host, Process,
    Program, StackInputs,
};

// MOCK CODE EXECUTOR
// ================================================================================================

/// Helper for executing arbitrary code within arbitrary hosts.
pub struct CodeExecutor<H> {
    host: H,
    stack_inputs: Option<StackInputs>,
    advice_inputs: AdviceInputs,
    file_path: Option<PathBuf>,
    imports: String,
}

impl<H: Host> CodeExecutor<H> {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    pub fn new(host: H) -> Self {
        Self {
            host,
            stack_inputs: None,
            advice_inputs: AdviceInputs::default(),
            file_path: None,
            imports: String::new(),
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
    ///
    /// If a module file path was set, its contents will be inserted between `self.imports` and
    /// `code` before execution.
    /// Otherwise, `self.imports` and `code` will be concatenated and the result will be executed.
    pub fn run(self, code: &str) -> Result<Process<H>, ExecutionError> {
        let assembler = TransactionKernel::assembler();
        let code = match &self.file_path {
            Some(file_path) => load_file_with_code(&self.imports, code, file_path.to_path_buf()),
            None => format!("{}{code}", self.imports),
        };

        let program = assembler.compile(code).unwrap();
        self.execute_program(program)
    }

    pub fn execute_program(self, program: Program) -> Result<Process<H>, ExecutionError> {
        let mut process = Process::new(
            program.kernel().clone(),
            self.stack_inputs.unwrap_or_default(),
            self.host,
            ExecutionOptions::default(),
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
        let host = DefaultHost::new(adv_provider);
        CodeExecutor::new(host)
    }
}

// UTILS
// ================================================================================================

/// Loads the specified file and append `code` into its end.
#[cfg(feature = "std")]
pub fn load_file_with_code(imports: &str, code: &str, assembly_file: PathBuf) -> String {
    use alloc::string::String;
    use std::fs::File;

    let mut module = String::new();
    File::open(assembly_file).unwrap().read_to_string(&mut module).unwrap();
    let complete_code = format!("{imports}{module}{code}");

    // This hack is going around issue #686 on miden-vm
    complete_code.replace("export", "proc")
}
