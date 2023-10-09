#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

use assembly::{
    ast::{ModuleAst, ProgramAst},
    Assembler, AssemblyContext,
};
use crypto::{
    hash::rpo::{Rpo256 as Hasher, RpoDigest as Digest},
    merkle::{MerkleError, Mmr, TieredSmt},
    utils::{
        collections::Vec,
        string::{String, ToString},
    },
    Felt, StarkField, Word, WORD_SIZE, ZERO,
};
use vm_core::code_blocks::CodeBlock;
use vm_processor::AdviceInputs;

pub mod accounts;

mod advice;
use advice::{AdviceInputsBuilder, ToAdviceInputs};

pub mod assets;
pub mod notes;

pub mod block;
pub use block::BlockHeader;

pub mod chain;
pub use chain::ChainMmr;

mod errors;
pub use errors::{
    AccountError, AssetError, ExecutedTransactionError, NoteError, PreparedTransactionError,
    TransactionResultError, TransactionWitnessError,
};

pub mod transaction;
