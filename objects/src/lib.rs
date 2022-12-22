#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

use crypto::{
    hash::rpo::{Rpo256 as Hasher, RpoDigest as Digest},
    utils::collections::Vec,
    Felt, StarkField, Word, WORD_SIZE, ZERO,
};

mod accounts;
pub use accounts::AccountId;

pub mod assets;
pub mod notes;

mod errors;
pub use errors::{AccountError, AssetError, NoteError};
