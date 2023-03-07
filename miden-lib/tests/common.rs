pub use crypto::{
    hash::rpo::{Rpo256 as Hasher, RpoDigest as Digest},
    FieldElement, StarkField,
};
use miden_lib::MidenLib;
pub use processor::{
    math::Felt, AdviceInputs, AdviceProvider, ExecutionError, MemAdviceProvider, Process,
    StackInputs, Word,
};

use std::{env, fs::File, io::Read, path::Path};

/// Loads the specified file and append `code` into its end.
pub fn load_file_with_code<T>(code: T, dir: &str, file: &str) -> String
where
    T: AsRef<str>,
{
    let assembly_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("asm").join(dir).join(file);

    let mut complete_code = String::new();
    File::open(assembly_file).unwrap().read_to_string(&mut complete_code).unwrap();

    complete_code.push_str(code.as_ref());

    // This hack is going around issue #686 on miden-vm
    complete_code.replace("export", "proc")
}

/// Inject `code` along side the specified file and run it
pub fn run_within_tx_kernel<A, T>(
    code: T,
    stack_inputs: StackInputs,
    adv: A,
    dir: &str,
    file: &str,
) -> Process<A>
where
    A: AdviceProvider,
    T: AsRef<str>,
{
    let assembler = assembly::Assembler::default()
        .with_library(&MidenLib::default())
        .expect("failed to load stdlib");

    let code = load_file_with_code(code, dir, file);
    let program = assembler.compile(code).unwrap();

    let mut process = Process::new(program.kernel().clone(), stack_inputs, adv);
    process.execute(&program).unwrap();

    process
}
