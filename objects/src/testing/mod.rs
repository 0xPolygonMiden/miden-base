use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use vm_core::Word;

pub mod account;
pub mod account_code;
pub mod account_component;
pub mod account_id;
pub mod assets;
pub mod block;
pub mod constants;
pub mod notes;
pub mod storage;

/// Converts a word to MASM
pub fn prepare_word(word: &Word) -> String {
    word.iter().map(|x| x.as_int().to_string()).collect::<Vec<_>>().join(".")
}
