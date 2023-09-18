#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

use assembly::{
    ast::{ModuleAst, ProgramAst},
    Assembler, AssemblyContext, LibraryPath, Module,
};
use crypto::{
    hash::rpo::{Rpo256 as Hasher, RpoDigest as Digest},
    merkle::{MerkleError, MerkleStore, Mmr, TieredSmt},
    utils::{
        collections::{BTreeMap, Vec},
        string::{String, ToString},
    },
    Felt, StarkField, Word, WORD_SIZE, ZERO,
};
use vm_core::code_blocks::CodeBlock;
use vm_processor::{AdviceInputs, StackOutputs};

mod accounts;
pub use accounts::{
    validate_account_seed, Account, AccountCode, AccountDelta, AccountId, AccountStorage,
    AccountStub, AccountType, AccountVault, StorageItem,
};

mod advice;
use advice::{AdviceInputsBuilder, ToAdviceInputs};

pub mod assets;
pub mod notes;

pub mod block;
pub use block::BlockHeader;

#[cfg(feature = "mock")]
pub mod builder;

pub mod chain;
pub use chain::ChainMmr;

mod errors;
pub use errors::{
    AccountError, AssetError, ExecutedTransactionError, NoteError, PreparedTransactionError,
    TransactionResultError, TransactionWitnessError,
};

mod result;
pub use result::TryFromVmResult;

pub mod transaction;

#[cfg(feature = "mock")]
pub mod mock;
