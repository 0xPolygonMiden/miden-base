use alloc::string::ToString;
use core::fmt;

use miden_crypto::{merkle::LeafIndex, utils::ByteWriter};
use vm_core::{
    utils::{ByteReader, Deserializable, Serializable},
    Felt,
};
use vm_processor::DeserializationError;

use super::account_id;
use crate::{
    accounts::{account_id::validate_first_felt, AccountStorageMode, AccountType, AccountVersion},
    AccountError, ACCOUNT_TREE_DEPTH,
};

// ACCOUNT ID PREFIX
// ================================================================================================

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AccountIdPrefix {
    first_felt: Felt,
}

impl AccountIdPrefix {
    pub fn new_unchecked(first_felt: Felt) -> Self {
        AccountIdPrefix { first_felt }
    }

    pub fn new(first_felt: Felt) -> Result<Self, AccountError> {
        validate_first_felt(first_felt)?;

        Ok(AccountIdPrefix { first_felt })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    pub const fn account_type(&self) -> AccountType {
        account_id::extract_type(self.first_felt.as_int())
    }

    /// Returns true if an account with this ID is a faucet (can issue assets).
    pub fn is_faucet(&self) -> bool {
        self.account_type().is_faucet()
    }

    /// Returns true if an account with this ID is a regular account.
    pub fn is_regular_account(&self) -> bool {
        self.account_type().is_regular_account()
    }

    pub fn storage_mode(&self) -> AccountStorageMode {
        account_id::extract_storage_mode(self.first_felt.as_int())
            .expect("account id prefix should have been constructed with a valid storage mode")
    }

    /// Returns true if an account with this ID is a public account.
    pub fn is_public(&self) -> bool {
        self.storage_mode() == AccountStorageMode::Public
    }

    pub fn version(&self) -> AccountVersion {
        account_id::extract_version(self.first_felt.as_int())
            .expect("account id prefix should have been constructed with a valid version")
    }
}

// CONVERSIONS FROM ACCOUNT ID PREFIX
// ================================================================================================

impl From<AccountIdPrefix> for Felt {
    fn from(id: AccountIdPrefix) -> Self {
        id.first_felt
    }
}

impl From<AccountIdPrefix> for [u8; 8] {
    fn from(id: AccountIdPrefix) -> Self {
        let mut result = [0_u8; 8];
        result[..8].copy_from_slice(&id.first_felt.as_int().to_le_bytes());
        result
    }
}

impl From<AccountIdPrefix> for u64 {
    fn from(id: AccountIdPrefix) -> Self {
        id.first_felt.as_int()
    }
}

/// Account IDs are used as indexes in the account database, which is a tree of depth 64.
impl From<AccountIdPrefix> for LeafIndex<ACCOUNT_TREE_DEPTH> {
    fn from(id: AccountIdPrefix) -> Self {
        LeafIndex::new_max_depth(id.first_felt.as_int())
    }
}

// CONVERSIONS TO ACCOUNT ID PREFIX
// ================================================================================================

impl TryFrom<[u8; 8]> for AccountIdPrefix {
    type Error = AccountError;

    // Expects little-endian byte order
    fn try_from(value: [u8; 8]) -> Result<Self, Self::Error> {
        let element =
            Felt::try_from(&value[..8]).map_err(AccountError::AccountIdInvalidFieldElement)?;
        Self::new(element)
    }
}

impl TryFrom<u64> for AccountIdPrefix {
    type Error = AccountError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let element = Felt::try_from(value.to_le_bytes().as_slice())
            .map_err(AccountError::AccountIdInvalidFieldElement)?;
        Self::new(element)
    }
}

impl TryFrom<Felt> for AccountIdPrefix {
    type Error = AccountError;

    fn try_from(element: Felt) -> Result<Self, Self::Error> {
        Self::new(element)
    }
}

// COMMON TRAIT IMPLS
// ================================================================================================

impl PartialOrd for AccountIdPrefix {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AccountIdPrefix {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.first_felt.as_int().cmp(&other.first_felt.as_int())
    }
}

impl fmt::Display for AccountIdPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.first_felt.as_int())
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountIdPrefix {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let bytes: [u8; 8] = (*self).into();
        bytes.write_into(target);
    }

    fn get_size_hint(&self) -> usize {
        // TODO: Turn into constant?
        8
    }
}

impl Deserializable for AccountIdPrefix {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        <[u8; 8]>::read_from(source)?
            .try_into()
            .map_err(|err: AccountError| DeserializationError::InvalidValue(err.to_string()))
    }
}
