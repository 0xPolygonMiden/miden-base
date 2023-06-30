#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

use assembly::{
    ast::{ModuleAst, ProgramAst},
    Assembler, AssemblyContext, AssemblyContextType, LibraryPath, Module,
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
use miden_core::code_blocks::CodeBlock;
use miden_processor::{AdviceInputs, AdviceProvider, StackOutputs};

mod accounts;
pub use accounts::{
    Account, AccountCode, AccountDelta, AccountId, AccountStorage, AccountStub, AccountType,
    AccountVault, StorageItem,
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
pub use errors::{
    AccountError, AssetError, NoteError, TransactionResultError, TransactionWitnessError,
};

mod result;
pub use result::TryFromVmResult;

pub mod transaction;

#[cfg(any(test, feature = "testing"))]
pub mod mock;

// CONSTANTS
// -----------------------------------------------------------------------------------------------

/// An empty word.
const EMPTY_WORD: Word = [ZERO, ZERO, ZERO, ZERO];
