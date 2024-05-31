use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use vm_core::{code_blocks::CodeBlock, Operation, Program, Word, ZERO};

pub mod account;
pub mod account_code;
pub mod account_id;
pub mod assets;
pub mod block;
pub mod constants;
pub mod notes;
pub mod storage;

pub fn build_dummy_tx_program() -> Program {
    let operations = vec![Operation::Push(ZERO), Operation::Drop];
    let span = CodeBlock::new_span(operations);
    Program::new(span)
}

/// Converts a word to MASM
pub fn prepare_word(word: &Word) -> String {
    word.iter().map(|x| x.as_int().to_string()).collect::<Vec<_>>().join(".")
}
