use alloc::string::{String, ToString};
use core::fmt;

use miden_crypto::utils::ByteWriter;
use vm_core::{
    utils::{ByteReader, Deserializable, Serializable},
    Felt,
};
use vm_processor::DeserializationError;

use super::id_v0;
use crate::{
    accounts::{
        account_id::id_v0::validate_prefix, AccountIdVersion, AccountStorageMode, AccountType,
    },
    errors::AccountIdError,
};

// ACCOUNT ID PREFIX VERSION 0
// ================================================================================================

/// The prefix of an [`AccountId`][id], i.e. its first field element.
///
/// See the type's documentation for details.
///
/// The serialization formats of [`AccountIdPrefix`] and [`AccountId`][id] are compatible. In
/// particular, a prefix can be deserialized from the serialized bytes of a full id.
///
/// [id]: crate::accounts::AccountId
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AccountIdPrefixV0 {
    prefix: Felt,
}

impl AccountIdPrefixV0 {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// The serialized size of an [`AccountIdPrefix`] in bytes.
    pub const SERIALIZED_SIZE: usize = 8;

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Constructs a new [`AccountIdPrefix`] from the given `prefix` without checking its
    /// validity.
    ///
    /// # Warning
    ///
    /// Validity of the ID prefix must be ensured by the caller. An invalid ID may lead to panics.
    ///
    /// # Panics
    ///
    /// If debug_assertions are enabled (e.g. in debug mode), this function panics if the given
    /// felt is invalid according to the constraints in the
    /// [`AccountId`](crate::accounts::AccountId) documentation.
    pub fn new_unchecked(prefix: Felt) -> Self {
        // Panic on invalid felts in debug mode.
        if cfg!(debug_assertions) {
            validate_prefix(prefix)
                .expect("AccountIdPrefix::new_unchecked called with invalid prefix");
        }

        AccountIdPrefixV0 { prefix }
    }

    /// Constructs a new [`AccountIdPrefix`] from the given `prefix` and checks its validity.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints of the prefix are not met. See the
    /// [`AccountId`](crate::accounts::AccountId) type documentation for details.
    pub fn new(prefix: Felt) -> Result<Self, AccountIdError> {
        validate_prefix(prefix)?;

        Ok(AccountIdPrefixV0 { prefix })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the [`Felt`] that represents this prefix.
    pub const fn as_felt(&self) -> Felt {
        self.prefix
    }

    /// Returns the prefix as a [`u64`].
    pub const fn as_u64(&self) -> u64 {
        self.prefix.as_int()
    }

    /// Returns the type of this account ID.
    pub const fn account_type(&self) -> AccountType {
        id_v0::extract_type(self.prefix.as_int())
    }

    /// Returns true if an account with this ID is a faucet (can issue assets).
    pub fn is_faucet(&self) -> bool {
        self.account_type().is_faucet()
    }

    /// Returns true if an account with this ID is a regular account.
    pub fn is_regular_account(&self) -> bool {
        self.account_type().is_regular_account()
    }

    /// Returns the storage mode of this account ID.
    pub fn storage_mode(&self) -> AccountStorageMode {
        id_v0::extract_storage_mode(self.prefix.as_int())
            .expect("account ID prefix should have been constructed with a valid storage mode")
    }

    /// Returns true if an account with this ID is a public account.
    pub fn is_public(&self) -> bool {
        self.storage_mode() == AccountStorageMode::Public
    }

    /// Returns the version of this account ID.
    pub fn version(&self) -> AccountIdVersion {
        id_v0::extract_version(self.prefix.as_int())
            .expect("account ID prefix should have been constructed with a valid version")
    }

    /// Returns the prefix as a big-endian, hex-encoded string.
    pub fn to_hex(self) -> String {
        format!("0x{:016x}", self.prefix.as_int())
    }
}

// CONVERSIONS FROM ACCOUNT ID PREFIX
// ================================================================================================

impl From<AccountIdPrefixV0> for Felt {
    fn from(id: AccountIdPrefixV0) -> Self {
        id.prefix
    }
}

impl From<AccountIdPrefixV0> for [u8; 8] {
    fn from(id: AccountIdPrefixV0) -> Self {
        let mut result = [0_u8; 8];
        result[..8].copy_from_slice(&id.prefix.as_int().to_be_bytes());
        result
    }
}

impl From<AccountIdPrefixV0> for u64 {
    fn from(id: AccountIdPrefixV0) -> Self {
        id.prefix.as_int()
    }
}

// CONVERSIONS TO ACCOUNT ID PREFIX
// ================================================================================================

impl TryFrom<[u8; 8]> for AccountIdPrefixV0 {
    type Error = AccountIdError;

    /// Tries to convert a byte array in big-endian order to an [`AccountIdPrefix`].
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints of the prefix are not met. See the
    /// [`AccountId`](crate::accounts::AccountId) type documentation for details.
    fn try_from(mut value: [u8; 8]) -> Result<Self, Self::Error> {
        // Felt::try_from expects little-endian order.
        value.reverse();

        Felt::try_from(value.as_slice())
            .map_err(AccountIdError::AccountIdInvalidPrefixFieldElement)
            .and_then(Self::new)
    }
}

impl TryFrom<u64> for AccountIdPrefixV0 {
    type Error = AccountIdError;

    /// Tries to convert a `u64` into an [`AccountIdPrefix`].
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints of the prefix are not met. See the
    /// [`AccountId`](crate::accounts::AccountId) type documentation for details.
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let element = Felt::try_from(value.to_le_bytes().as_slice())
            .map_err(AccountIdError::AccountIdInvalidPrefixFieldElement)?;
        Self::new(element)
    }
}

impl TryFrom<Felt> for AccountIdPrefixV0 {
    type Error = AccountIdError;

    /// Returns an [`AccountIdPrefix`] instantiated with the provided field .
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints of the prefix are not met. See the
    /// [`AccountId`](crate::accounts::AccountId) type documentation for details.
    fn try_from(element: Felt) -> Result<Self, Self::Error> {
        Self::new(element)
    }
}

// COMMON TRAIT IMPLS
// ================================================================================================

impl PartialOrd for AccountIdPrefixV0 {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AccountIdPrefixV0 {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.prefix.as_int().cmp(&other.prefix.as_int())
    }
}

impl fmt::Display for AccountIdPrefixV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountIdPrefixV0 {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let bytes: [u8; 8] = (*self).into();
        bytes.write_into(target);
    }

    fn get_size_hint(&self) -> usize {
        Self::SERIALIZED_SIZE
    }
}

impl Deserializable for AccountIdPrefixV0 {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        <[u8; 8]>::read_from(source)?
            .try_into()
            .map_err(|err: AccountIdError| DeserializationError::InvalidValue(err.to_string()))
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accounts::{AccountId, AccountIdPrefix},
        testing::account_id::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
            ACCOUNT_ID_OFF_CHAIN_SENDER, ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        },
    };

    #[test]
    fn test_account_id_prefix_conversion_roundtrip() {
        for (idx, account_id) in [
            ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
            ACCOUNT_ID_OFF_CHAIN_SENDER,
        ]
        .into_iter()
        .enumerate()
        {
            let full_id = AccountId::try_from(account_id).unwrap();
            let prefix = full_id.prefix();
            assert_eq!(
                prefix,
                AccountIdPrefix::read_from_bytes(&prefix.to_bytes()).unwrap(),
                "failed in {idx}"
            );
        }
    }
}
