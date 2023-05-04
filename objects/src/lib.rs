#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

use crypto::{
    hash::rpo::{Rpo256 as Hasher, RpoDigest as Digest},
    merkle::Mmr,
    utils::{
        collections::{BTreeSet, Vec},
        string::{String, ToString},
    },
    Felt, StarkField, Word, WORD_SIZE, ZERO,
};

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
