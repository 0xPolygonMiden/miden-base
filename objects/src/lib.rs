#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

use assembly::{
    Assembler, AssemblyContext, AssemblyContextType, LibraryPath, Module, ModuleAst, ProgramAst,
};
use crypto::{
    hash::rpo::{Rpo256 as Hasher, RpoDigest as Digest},
    merkle::{MerkleError, Mmr},
    utils::{
        collections::Vec,
        string::{String, ToString},
    },
    Felt, StarkField, Word, WORD_SIZE, ZERO,
};
use miden_core::code_blocks::CodeBlock;

mod accounts;
pub use accounts::{
    Account, AccountCode, AccountId, AccountStorage, AccountType, AccountVault, StorageItem,
};

mod advice;
use advice::{AdviceInputsBuilder, ToAdviceInputs};

pub mod assets;
pub mod notes;

pub mod block;
pub use block::BlockHeader;

pub mod chain;
pub use chain::ChainMmr;

mod errors;
pub use errors::{AccountError, AssetError, NoteError};

pub mod transaction;
