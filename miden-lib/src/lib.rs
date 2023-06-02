#![cfg_attr(not(feature = "std"), no_std)]

use assembly::{
    Assembler, AssemblyContext, Deserializable, Library, LibraryNamespace, LibraryPath,
    MaslLibrary, Module, ModuleAst, ProgramAst, Version,
};
use miden_stdlib::StdLibrary;
use vm_core::{code_blocks::CodeBlock, Kernel};

pub mod memory;

// STANDARD LIBRARY
// ================================================================================================

pub struct MidenLib {
    contents: MaslLibrary,
}

impl Default for MidenLib {
    fn default() -> Self {
        let bytes = include_bytes!("../assets/miden.masl");
        let contents = MaslLibrary::read_from_bytes(bytes).expect("failed to read std masl!");
        Self { contents }
    }
}

impl Library for MidenLib {
    type ModuleIterator<'a> = <MaslLibrary as Library>::ModuleIterator<'a>;

    fn root_ns(&self) -> &LibraryNamespace {
        self.contents.root_ns()
    }

    fn version(&self) -> &Version {
        self.contents.version()
    }

    fn modules(&self) -> Self::ModuleIterator<'_> {
        self.contents.modules()
    }
}

// TRANSACTION KERNEL
// ================================================================================================

pub struct TransactionKernel;

impl TransactionKernel {
    // TRANSACTION KERNEL METHODS
    // --------------------------------------------------------------------------------------------
    pub fn kernel() -> (Kernel, ModuleAst) {
        let kernel_src = include_str!("../asm/sat/kernel.masm");
        let kernel_ast = ModuleAst::parse(kernel_src).expect("kernel is well formed");
        let kernel_module =
            Module::new(LibraryPath::new("kernel").expect("kernel path is correct"), kernel_ast);
        let kernel_proc_digests = Self::assembler()
            .compile_module(
                &kernel_module,
                &mut AssemblyContext::new(assembly::AssemblyContextType::Module),
            )
            .expect("kernel is well formed");
        (Kernel::new(&kernel_proc_digests), kernel_module.ast)
    }

    // TRANSACTION KERNEL SECTIONS
    // --------------------------------------------------------------------------------------------
    /// Retruns a [CodeBlock] which encodes the transaction kernel prologue.
    pub fn prologue() -> CodeBlock {
        let prologue_ast = ProgramAst::parse(
            "\
        use.miden::sat::prologue

        begin
            exec.prologue::prepare_transaction
        end
        ",
        )
        .expect("failed to read masm!");
        Self::compile_kernel_section(prologue_ast)
    }

    /// Returns a [CodeBlock] which encodes the transaction kernel epilogue.
    pub fn epilogue() -> CodeBlock {
        let epilogue_ast = ProgramAst::parse(
            "\
        use.miden::sat::epilogue

        begin
            exec.epilogue::finalize_transaction
        end",
        )
        .expect("failed to read masm!");
        Self::compile_kernel_section(epilogue_ast)
    }

    /// Returns a [CodeBlock] which encodes the transaction kernel note setup script.
    pub fn note_setup() -> CodeBlock {
        let note_setup_ast = ProgramAst::parse(
            "\
        use.miden::sat::note_setup

        begin
            exec.note_setup::prepare_note
        end
        ",
        )
        .expect("failed to read masm!");
        Self::compile_kernel_section(note_setup_ast)
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------
    fn compile_kernel_section(program_ast: ProgramAst) -> CodeBlock {
        Self::assembler()
            .compile_in_context(
                program_ast,
                &mut AssemblyContext::new(assembly::AssemblyContextType::Program),
            )
            .expect("failed to compile program!")
    }

    fn assembler() -> Assembler {
        Assembler::default()
            .with_library(&MidenLib::default())
            .expect("library is valid")
            .with_library(&StdLibrary::default())
            .expect("library is valid")
    }
}
// TEST
// ================================================================================================

#[test]
fn test_compile() {
    let path = "miden::sat::layout::get_consumed_note_ptr";
    let miden = MidenLib::default();
    let exists = miden.modules().any(|module| {
        module
            .ast
            .procs()
            .iter()
            .any(|proc| module.path.append(&proc.name).unwrap().as_str() == path)
    });

    assert!(exists);
}
