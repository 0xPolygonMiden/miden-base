#[cfg(not(target_family = "wasm"))]
use miden_lib::transaction::TransactionKernel;
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
    ///
    /// If a module file path was set, its contents will be inserted between `self.imports` and
    /// `code` before execution.
    /// Otherwise, `self.imports` and `code` will be concatenated and the result will be executed.
    pub fn run(self, code: &str) -> Result<Process<H>, ExecutionError> {
        let assembler = TransactionKernel::assembler().with_debug_mode(true);
        let program = assembler.compile(code).unwrap();
        self.execute_program(&program)
    }

    pub fn execute_program(self, program: &Program) -> Result<Process<H>, ExecutionError> {
        let mut process = Process::new_debug(
            program.kernel().clone(),
            self.stack_inputs.unwrap_or_default(),
            self.host,
        );
        process.execute(program)?;

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
