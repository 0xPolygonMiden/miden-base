use alloc::string::String;
use core::fmt;

use miden_objects::{accounts::AccountStorage, AssetError, Digest};

// TRANSACTION KERNEL ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TransactionKernelError {
    InvalidStorageSlotIndex(u64),
    MalformedAssetOnAccountVaultUpdate(AssetError),
    MissingStorageSlotValue(u8, String),
    UnknownAccountProcedure(Digest),
}

impl fmt::Display for TransactionKernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStorageSlotIndex(index) => {
                let num_slots = AccountStorage::NUM_STORAGE_SLOTS;
                write!(f, "storage slot index {index} is invalid, must be smaller than {num_slots}")
            },
            Self::MalformedAssetOnAccountVaultUpdate(err) => {
                write!(f, "malformed asset during account vault update: {err}")
            },
            Self::MissingStorageSlotValue(index, err) => {
                write!(f, "value for storage slot {index} could not be found: {err}")
            },
            Self::UnknownAccountProcedure(proc_root) => {
                write!(f, "account procedure with root {proc_root} is not in the advice provider")
            },
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionKernelError {}

// TRANSACTION EVENT PARSING ERROR
// ================================================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TransactionEventParsingError {
    InvalidTransactionEvent(u32),
    NotTransactionEvent(u32),
}

impl fmt::Display for TransactionEventParsingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransactionEvent(event_id) => {
                write!(f, "event {event_id} is not a valid transaction kernel event")
            },
            Self::NotTransactionEvent(event_id) => {
                write!(f, "event {event_id} is not a transaction kernel event")
            },
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransactionEventParsingError {}
