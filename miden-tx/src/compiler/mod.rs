use miden_objects::{
    assembly::{Assembler, AssemblyContext, ModuleAst, ProgramAst},
    transaction::{InputNotes, TransactionScript},
    Felt, NoteError, TransactionScriptError, Word,
};

use super::{
    AccountCode, AccountId, BTreeMap, CodeBlock, Digest, NoteScript, Program,
    TransactionCompilerError, TransactionKernel,
};

// TRANSACTION COMPILER
// ================================================================================================

/// The transaction compiler is responsible for building executable programs for Miden rollup
/// transactions.
///
/// The generated programs can then be executed on the Miden VM to update the states of accounts
/// involved in these transactions.
///
/// In addition to transaction compilation, transaction compiler provides methods which can be
/// used to compile Miden account code and note scripts.
pub struct TransactionCompiler {
    assembler: Assembler,
    account_procedures: BTreeMap<AccountId, Vec<Digest>>,
    kernel_main: CodeBlock,
}

impl TransactionCompiler {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new [TransactionCompiler].
    pub fn new() -> TransactionCompiler {
        let assembler = TransactionKernel::assembler();

        // compile transaction kernel main
        let main_ast = TransactionKernel::main().expect("main is well formed");
        let kernel_main = assembler
            .compile_in_context(&main_ast, &mut AssemblyContext::for_program(Some(&main_ast)))
            .expect("main is well formed");

        TransactionCompiler {
            assembler,
            account_procedures: BTreeMap::default(),
            kernel_main,
        }
    }

    // ACCOUNT CODE AND NOTE SCRIPT COMPILERS
    // --------------------------------------------------------------------------------------------

    /// Compiles the provided module into [AccountCode] and associates the resulting procedures
    /// with the specified account ID.
    pub fn load_account(
        &mut self,
        account_id: AccountId,
        account_code: ModuleAst,
    ) -> Result<AccountCode, TransactionCompilerError> {
        let account_code = AccountCode::new(account_code, &self.assembler)
            .map_err(TransactionCompilerError::LoadAccountFailed)?;
        self.account_procedures.insert(account_id, account_code.procedures().to_vec());
        Ok(account_code)
    }

    /// Loads the provided account interface (vector of procedure digests) into this compiler.
    /// Returns the old account interface if it previously existed.
    pub fn load_account_interface(
        &mut self,
        account_id: AccountId,
        procedures: Vec<Digest>,
    ) -> Option<Vec<Digest>> {
        self.account_procedures.insert(account_id, procedures)
    }

    /// Compiles the provided program into the [NoteScript] and checks (to the extent possible)
    /// if a note could be executed against all accounts with the specified interfaces.
    pub fn compile_note_script(
        &mut self,
        note_script_ast: ProgramAst,
        target_account_proc: Vec<ScriptTarget>,
    ) -> Result<NoteScript, TransactionCompilerError> {
        let (note_script, code_block) =
            NoteScript::new(note_script_ast, &self.assembler).map_err(|err| match err {
                NoteError::ScriptCompilationError(err) => {
                    TransactionCompilerError::CompileNoteScriptFailed(err)
                },
                _ => TransactionCompilerError::NoteScriptError(err),
            })?;
        for note_target in target_account_proc.into_iter() {
            verify_program_account_compatibility(
                &code_block,
                &self.get_target_interface(note_target)?,
                ScriptType::NoteScript,
            )?;
        }

        Ok(note_script)
    }

    /// Constructs a [TransactionScript] by compiling the provided source code and checking the
    /// compatibility of the resulting program with the target account interfaces.
    pub fn compile_tx_script<T>(
        &mut self,
        tx_script_ast: ProgramAst,
        tx_script_inputs: T,
        target_account_proc: Vec<ScriptTarget>,
    ) -> Result<TransactionScript, TransactionCompilerError>
    where
        T: IntoIterator<Item = (Word, Vec<Felt>)>,
    {
        let (tx_script, code_block) =
            TransactionScript::new(tx_script_ast, tx_script_inputs, &mut self.assembler).map_err(
                |e| match e {
                    TransactionScriptError::ScriptCompilationError(asm_error) => {
                        TransactionCompilerError::CompileTxScriptFailed(asm_error)
                    },
                },
            )?;
        for target in target_account_proc.into_iter() {
            verify_program_account_compatibility(
                &code_block,
                &self.get_target_interface(target)?,
                ScriptType::TransactionScript,
            )?;
        }
        Ok(tx_script)
    }

    // TRANSACTION PROGRAM BUILDER
    // --------------------------------------------------------------------------------------------
    /// Compiles a transaction which executes the provided notes and an optional tx script against
    /// the specified account. Returns the the compiled transaction program.
    ///
    /// The account is assumed to have been previously loaded into this compiler.
    pub fn compile_transaction(
        &mut self,
        account_id: AccountId,
        notes: &InputNotes,
        tx_script: Option<&ProgramAst>,
    ) -> Result<Program, TransactionCompilerError> {
        // Fetch the account interface from the `account_procedures` map. Return an error if the
        // interface is not found.
        let target_account_interface = self
            .account_procedures
            .get(&account_id)
            .cloned()
            .ok_or(TransactionCompilerError::AccountInterfaceNotFound(account_id))?;

        // Transaction must contain at least one input note or a transaction script
        if notes.is_empty() && tx_script.is_none() {
            return Err(TransactionCompilerError::NoTransactionDriver);
        }

        // Create the [AssemblyContext] for compilation of notes scripts and the transaction script
        let mut assembly_context = AssemblyContext::for_program(None);

        // Compile note scripts
        let note_script_programs =
            self.compile_notes(&target_account_interface, notes, &mut assembly_context)?;

        // Compile the transaction script
        let tx_script_program = match tx_script {
            Some(tx_script) => Some(self.compile_tx_script_program(
                tx_script,
                &mut assembly_context,
                target_account_interface,
            )?),
            None => None,
        };

        // Create [CodeBlockTable] from [AssemblyContext]
        let mut cb_table = self
            .assembler
            .build_cb_table(assembly_context)
            .map_err(TransactionCompilerError::BuildCodeBlockTableFailed)?;

        // insert note roots into [CodeBlockTable]
        note_script_programs.into_iter().for_each(|note_root| {
            cb_table.insert(note_root);
        });

        // insert transaction script into [CodeBlockTable]
        if let Some(tx_script_program) = tx_script_program {
            cb_table.insert(tx_script_program);
        }

        // Create transaction program with kernel
        let program = Program::with_kernel(
            self.kernel_main.clone(),
            self.assembler.kernel().clone(),
            cb_table,
        );

        // Create compiled transaction
        Ok(program)
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    /// Compiles the provided notes into [CodeBlock]s (programs) and verifies that each note is
    /// compatible with the target account interfaces. Returns a vector of the compiled note
    /// programs.
    fn compile_notes(
        &mut self,
        target_account_interface: &[Digest],
        notes: &InputNotes,
        assembly_context: &mut AssemblyContext,
    ) -> Result<Vec<CodeBlock>, TransactionCompilerError> {
        let mut note_programs = Vec::new();

        // Create and verify note programs. Note programs are verified against the target account.
        for recorded_note in notes.iter() {
            let note_program = self
                .assembler
                .compile_in_context(recorded_note.note().script().code(), assembly_context)
                .map_err(TransactionCompilerError::CompileNoteScriptFailed)?;
            verify_program_account_compatibility(
                &note_program,
                target_account_interface,
                ScriptType::NoteScript,
            )?;
            note_programs.push(note_program);
        }

        Ok(note_programs)
    }

    /// Returns a [CodeBlock] of the compiled transaction script program.
    ///
    /// The transaction script compatibility is verified against the target account interface.
    fn compile_tx_script_program(
        &mut self,
        tx_script: &ProgramAst,
        assembly_context: &mut AssemblyContext,
        target_account_interface: Vec<Digest>,
    ) -> Result<CodeBlock, TransactionCompilerError> {
        let tx_script_code_block = self
            .assembler
            .compile_in_context(tx_script, assembly_context)
            .map_err(TransactionCompilerError::CompileTxScriptFailed)?;
        verify_program_account_compatibility(
            &tx_script_code_block,
            &target_account_interface,
            ScriptType::TransactionScript,
        )?;
        Ok(tx_script_code_block)
    }

    /// Returns the account interface associated with the provided [ScriptTarget].
    ///
    /// # Errors
    /// - If the account interface associated with the [AccountId] provided as a target can not be
    ///   found in the `account_procedures` map.
    fn get_target_interface(
        &self,
        target: ScriptTarget,
    ) -> Result<Vec<Digest>, TransactionCompilerError> {
        match target {
            ScriptTarget::AccountId(id) => self
                .account_procedures
                .get(&id)
                .cloned()
                .ok_or(TransactionCompilerError::AccountInterfaceNotFound(id)),
            ScriptTarget::Procedures(procs) => Ok(procs),
        }
    }
}

impl Default for TransactionCompiler {
    fn default() -> Self {
        Self::new()
    }
}

// TRANSACTION COMPILER HELPERS
// ------------------------------------------------------------------------------------------------

/// Verifies that the provided program is compatible with the target account interface.
///
/// This is achieved by checking that at least one execution branch in the program is compatible
/// with the target account interface.
///
/// # Errors
/// Returns an error if the program is not compatible with the target account interface.
fn verify_program_account_compatibility(
    program: &CodeBlock,
    target_account_interface: &[Digest],
    script_type: ScriptType,
) -> Result<(), TransactionCompilerError> {
    // collect call branches
    let branches = collect_call_branches(program);

    // if none of the branches are compatible with the target account, return an error
    if !branches.iter().any(|call_targets| {
        call_targets.iter().all(|target| target_account_interface.contains(target))
    }) {
        return match script_type {
            ScriptType::NoteScript => {
                Err(TransactionCompilerError::NoteIncompatibleWithAccountInterface(program.hash()))
            },
            ScriptType::TransactionScript => Err(
                TransactionCompilerError::TxScriptIncompatibleWithAccountInterface(program.hash()),
            ),
        };
    }

    Ok(())
}

/// Collect call branches by recursively traversing through program execution branches and
/// accumulating call targets.
fn collect_call_branches(code_block: &CodeBlock) -> Vec<Vec<Digest>> {
    let mut branches = vec![vec![]];
    recursively_collect_call_branches(code_block, &mut branches);
    branches
}

/// Generates a list of calls invoked in each execution branch of the provided code block.
fn recursively_collect_call_branches(code_block: &CodeBlock, branches: &mut Vec<Vec<Digest>>) {
    match code_block {
        CodeBlock::Join(block) => {
            recursively_collect_call_branches(block.first(), branches);
            recursively_collect_call_branches(block.second(), branches);
        },
        CodeBlock::Split(block) => {
            let current_len = branches.last().expect("at least one execution branch").len();
            recursively_collect_call_branches(block.on_false(), branches);

            // If the previous branch had additional calls we need to create a new branch
            if branches.last().expect("at least one execution branch").len() > current_len {
                branches.push(
                    branches.last().expect("at least one execution branch")[..current_len].to_vec(),
                );
            }

            recursively_collect_call_branches(block.on_true(), branches);
        },
        CodeBlock::Loop(block) => {
            recursively_collect_call_branches(block.body(), branches);
        },
        CodeBlock::Call(block) => {
            if block.is_syscall() {
                return;
            }

            branches
                .last_mut()
                .expect("at least one execution branch")
                .push(block.fn_hash());
        },
        CodeBlock::Span(_) => {},
        CodeBlock::Proxy(_) => {},
        CodeBlock::Dyn(_) => {},
    }
}

// SCRIPT TARGET
// ================================================================================================

/// The [ScriptTarget] enum is used to specify the target account interface for note and
/// transaction scripts.
///
/// This is specified as an account ID (for which the interface should be fetched) or a vector of
/// procedure digests which represents the account interface.
#[derive(Clone)]
pub enum ScriptTarget {
    AccountId(AccountId),
    Procedures(Vec<Digest>),
}

// SCRIPT TYPE
// ================================================================================================

enum ScriptType {
    NoteScript,
    TransactionScript,
}
