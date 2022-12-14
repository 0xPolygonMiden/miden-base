#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

use crypto::{hash::rpo::Rpo256 as Hasher, Felt, StarkField, Word, ZERO};

mod accounts;
pub use accounts::AccountId;

mod errors;
pub use errors::AccountError;
