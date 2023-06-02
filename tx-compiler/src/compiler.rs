use super::{
    AccountCode, AccountId, Assembler, AssemblyContext, AssemblyContextType, CodeBlock,
    CompiledTransaction, Digest, HashMap, Kernel, MidenLib, ModuleAst, Note, NoteScript, Operation,
    Program, ProgramAst, StdLibrary, TransactionError, TransactionKernel,
};

// TRANSACTION COMPILER
// ================================================================================================

pub struct TransactionComplier {
    assembler: Assembler,
    account_procedures: HashMap<AccountId, Vec<Digest>>,
    prologue: CodeBlock,
    epilogue: CodeBlock,
    note_setup: CodeBlock,
    tx_kernel: Kernel,
}

impl TransactionComplier {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    pub fn new() -> TransactionComplier {
        let (tx_kernel, tx_module) = TransactionKernel::kernel();
        let assembler = Assembler::default()
            .with_library(&MidenLib::default())
            .expect("library is well formed")
            .with_library(&StdLibrary::default())
            .expect("library is well formed")
            .with_kernel_module(tx_module)
            .expect("kernel is well formed");
        TransactionComplier {
            assembler,
            account_procedures: HashMap::default(),
            prologue: TransactionKernel::prologue(),
            epilogue: TransactionKernel::epilogue(),
            note_setup: TransactionKernel::note_setup(),
            tx_kernel,
        }
    }

    /// Compiles the provided module into [AccountCode] and associates the resulting procedures
    /// with the specified account ID.
    pub fn load_account(
        &mut self,
        account_id: AccountId,
        account_code: ModuleAst,
    ) -> Result<AccountCode, TransactionError> {
        let account_code = AccountCode::new(account_code, account_id, &mut self.assembler)
            .map_err(TransactionError::LoadAccountFailed)?;
        self.account_procedures.insert(account_id, account_code.procedures().to_vec());
        Ok(account_code)
    }

    /// Loads the provided account interface (vector of procedure digests) into the `account_procedures` map.
    /// Returns the old account interface if it previously existed.
    pub fn load_account_interface(
        &mut self,
        account_id: AccountId,
        procedures: Vec<Digest>,
    ) -> Option<Vec<Digest>> {
        self.account_procedures.insert(account_id, procedures)
    }

    /// Compiles the provided program into the [NoteScript] and checks (to the extent possible)
    /// if a note could be executed against an account with the specified interface.
    pub fn compile_note_script(
        &mut self,
        note_script_ast: ProgramAst,
        // TODO: Should this be optional - what if we don't know the target account interface.
        target_account_proc: Vec<NoteTarget>,
    ) -> Result<NoteScript, TransactionError> {
        let (note_script, code_block) = NoteScript::new(
            note_script_ast,
            &mut self.assembler,
            &mut AssemblyContext::new(AssemblyContextType::Program),
        )
        .map_err(|_| TransactionError::CompileNoteScriptFailed)?;

        let branches = collect_call_branches(&code_block);
        for note_target in target_account_proc.into_iter() {
            verify_note_account_compatibility(
                note_script.hash(),
                &branches,
                &self.get_target_interface(note_target)?,
            )?;
        }

        Ok(note_script)
    }

    /// Returns the account interface asscoiated with the provided [NoteTarget].
    ///
    /// # Errors
    /// - If the account interface associated with the [AccountId] provided as a target can not be
    ///   found in the `account_procedures` map.
    fn get_target_interface(&self, target: NoteTarget) -> Result<Vec<Digest>, TransactionError> {
        match target {
            NoteTarget::AccountId(id) => self
                .account_procedures
                .get(&id)
                .cloned()
                .ok_or(TransactionError::AccountInterfaceNotFound(id)),
            NoteTarget::Procedures(procs) => Ok(procs),
        }
    }

    /// Compiles a transaction which executes the provided notes against the specified account.
    ///
    /// The account is assumed to have been previously loaded into this compiler.
    pub fn compile_transaction(
        &mut self,
        account_id: AccountId,
        notes: Vec<Note>,
        tx_script: Option<ProgramAst>,
    ) -> Result<CompiledTransaction, TransactionError> {
        // Check if the account has been loaded into the [TransactionCompiler]
        if !self.account_procedures.contains_key(&account_id) {
            return Err(TransactionError::AccountInterfaceNotFound(account_id));
        }

        // Transaction must contain at least one input note
        if notes.is_empty() {
            return Err(TransactionError::NoNotesProvided);
        }

        // Create the [AssemblyContext] for compilation of the transaction program
        let mut assembly_context = AssemblyContext::new(AssemblyContextType::Program);

        // Create note tree
        let note_program_tree =
            self.create_note_program_tree(&account_id, &notes, &mut assembly_context)?;

        // Create the transaction script [CodeBlock].  If no tx_script is provided we use a
        // Noop span block.
        let tx_script = match tx_script {
            Some(tx_script) => self
                .assembler
                .compile_in_context(tx_script, &mut assembly_context)
                .map_err(TransactionError::CompileTxScriptFailed)?,
            None => CodeBlock::new_span(vec![Operation::Noop]),
        };

        // Merge transaction script code block and epilogue code block
        let tx_script_and_epilogue =
            CodeBlock::new_join([tx_script.clone(), self.epilogue.clone()]);

        // Merge prologue and note script tree
        let prologue_and_notes = CodeBlock::new_join([self.prologue.clone(), note_program_tree]);

        // Merge prologue, note tree, tx script and epilogue
        let program_root = CodeBlock::new_join([prologue_and_notes, tx_script_and_epilogue]);

        // Create [CodeBlockTable] from [AssemblyContext]
        let cb_table = self
            .assembler
            .build_cb_table(assembly_context)
            .map_err(TransactionError::BuildCodeBlockTableFailed)?;

        // Create transaction program
        let program = Program::with_kernel(program_root, self.tx_kernel.clone(), cb_table);

        // Create compiled transaction
        // TODO: we always have a digest for a transaction script (noop code block root if no user
        // tx scrip is provided) - should we we change the [CompiledTransaction] struct to take a
        // digest instead of a Option<Digest> fro the tx script.
        Ok(CompiledTransaction::new(account_id, notes, Some(tx_script.hash()), program))
    }

    /// Returns a [CodeBlock] which contains the note program tree.
    fn create_note_program_tree(
        &mut self,
        account_id: &AccountId,
        notes: &[Note],
        assembly_context: &mut AssemblyContext,
    ) -> Result<CodeBlock, TransactionError> {
        // Fetch target account interface
        let target_account_procs = self
            .account_procedures
            .get(account_id)
            .ok_or(TransactionError::AccountInterfaceNotFound(*account_id))?;

        // Create note programs
        let mut note_program_tree = notes
            .iter()
            .map(|note| {
                let note_root = self
                    .assembler
                    .compile_in_context(note.script().code().clone(), assembly_context)
                    .map_err(|_| TransactionError::CompileNoteScriptFailed)?;
                let note_branches = collect_call_branches(&note_root);
                verify_note_account_compatibility(
                    note_root.hash(),
                    &note_branches,
                    target_account_procs,
                )?;
                Ok(CodeBlock::new_join([self.note_setup.clone(), note_root]))
            })
            .collect::<Result<Vec<CodeBlock>, TransactionError>>()?;

        // Merge the note programs into a tree using join blcoks
        while note_program_tree.len() != 1 {
            // Pad note programs to an even number using a [Operation::Noop] span block
            if note_program_tree.len() % 2 != 0 {
                note_program_tree.push(CodeBlock::new_span(vec![Operation::Noop]));
            }

            note_program_tree = note_program_tree
                .chunks(2)
                .map(|code_blocks| {
                    CodeBlock::new_join([code_blocks[0].clone(), code_blocks[1].clone()])
                })
                .collect();
        }

        Ok(note_program_tree[0].clone())
    }
}

impl Default for TransactionComplier {
    fn default() -> Self {
        Self::new()
    }
}

// TRANSACTION COMPILER HELPERS
// ------------------------------------------------------------------------------------------------

/// Verifies that the provided note is compatible with the target account interface.
/// This is achieved by checking that at least one execution branch in the note script is compatible
/// with the target account interface.
///
/// # Errors
/// Returns an error if the note script is not compatible with the target account interface.
fn verify_note_account_compatibility(
    note_root: Digest,
    branches: &[Vec<Digest>],
    target_account_procs: &[Digest],
) -> Result<(), TransactionError> {
    // if none of the branches are compatible with the target account, return an error
    if !branches
        .iter()
        .any(|call_targets| call_targets.iter().all(|target| target_account_procs.contains(target)))
    {
        return Err(TransactionError::NoteIncompatibleWithAccountInterface(note_root));
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
        }
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
        }
        CodeBlock::Loop(block) => {
            recursively_collect_call_branches(block.body(), branches);
        }
        CodeBlock::Call(block) => {
            branches
                .last_mut()
                .expect("at least one execution branch")
                .push(block.fn_hash());
        }
        CodeBlock::Span(_) => {}
        CodeBlock::Proxy(_) => {}
    }
}

// NOTE TARGET
// ================================================================================================

#[derive(Clone)]
pub enum NoteTarget {
    AccountId(AccountId),
    Procedures(Vec<Digest>),
}
