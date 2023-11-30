#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

use vm_processor::AdviceInputs;

pub mod accounts;

mod advice;
use advice::AdviceInputsBuilder;
pub use advice::ToAdviceInputs;

pub mod assets;
pub mod notes;

pub mod block;
pub use block::BlockHeader;

pub mod chain;
pub use chain::ChainMmr;

pub mod transaction;

mod errors;
pub use errors::{
    AccountError, AssetError, ExecutedTransactionError, NoteError, PreparedTransactionError,
    TransactionResultError, TransactionScriptError, TransactionWitnessError,
};

// RE-EXPORTS
// ================================================================================================

pub use miden_crypto::hash::rpo::{Rpo256 as Hasher, RpoDigest as Digest};
pub use vm_core::{Felt, FieldElement, StarkField, Word, EMPTY_WORD, ONE, WORD_SIZE, ZERO};

pub mod assembly {
    pub use assembly::{
        ast::{AstSerdeOptions, ModuleAst, ProgramAst},
        Assembler, AssemblyContext, AssemblyError,
    };
    pub use vm_core::code_blocks::CodeBlock;
}

pub mod crypto {
    pub use miden_crypto::dsa;
    pub use miden_crypto::merkle;
    pub use miden_crypto::utils;
}

pub mod utils {
    pub use miden_crypto::utils::{format, vec};
    pub use vm_core::utils::{collections, string};
}
