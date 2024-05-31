use vm_core::{code_blocks::CodeBlock, Operation, Program, ZERO};

pub mod account;
pub mod account_id;
pub mod assets;
pub mod block;
pub mod constants;
pub mod storage;

pub fn build_dummy_tx_program() -> Program {
    let operations = vec![Operation::Push(ZERO), Operation::Drop];
    let span = CodeBlock::new_span(operations);
    Program::new(span)
}
