use alloc::string::{String, ToString};
use core::fmt;

use miden_crypto::utils::ByteWriter;
use vm_core::{
    utils::{ByteReader, Deserializable, Serializable},
    Felt,
};
use vm_processor::DeserializationError;

use super::account_id;
use crate::{
    accounts::{account_id::validate_prefix, AccountIdVersion, AccountStorageMode, AccountType},
    errors::AccountIdError,
};

// ACCOUNT ID PREFIX
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
pub struct AccountIdPrefix {
    prefix: Felt,
}

impl AccountIdPrefix {
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

        AccountIdPrefix { prefix }
    }

    /// Constructs a new [`AccountIdPrefix`] from the given `prefix` and checks its validity.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints of the prefix are not met. See the
    /// [`AccountId`](crate::accounts::AccountId) type documentation for details.
    pub fn new(prefix: Felt) -> Result<Self, AccountIdError> {
        validate_prefix(prefix)?;

        Ok(AccountIdPrefix { prefix })
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
        account_id::extract_type(self.prefix.as_int())
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
        account_id::extract_storage_mode(self.prefix.as_int())
            .expect("account ID prefix should have been constructed with a valid storage mode")
    }

    /// Returns true if an account with this ID is a public account.
    pub fn is_public(&self) -> bool {
        self.storage_mode() == AccountStorageMode::Public
    }

    /// Returns the version of this account ID.
    pub fn version(&self) -> AccountIdVersion {
        account_id::extract_version(self.prefix.as_int())
            .expect("account ID prefix should have been constructed with a valid version")
    }

    /// Returns the prefix as a big-endian, hex-encoded string.
    pub fn to_hex(&self) -> String {
        format!("0x{:016x}", self.prefix.as_int())
    }
}

// CONVERSIONS FROM ACCOUNT ID PREFIX
// ================================================================================================

impl From<AccountIdPrefix> for Felt {
    fn from(id: AccountIdPrefix) -> Self {
        id.prefix
    }
}

impl From<AccountIdPrefix> for [u8; 8] {
    fn from(id: AccountIdPrefix) -> Self {
        let mut result = [0_u8; 8];
        result[..8].copy_from_slice(&id.prefix.as_int().to_le_bytes());
        result
    }
}

impl From<AccountIdPrefix> for u64 {
    fn from(id: AccountIdPrefix) -> Self {
        id.prefix.as_int()
    }
}

// CONVERSIONS TO ACCOUNT ID PREFIX
// ================================================================================================

impl TryFrom<[u8; 8]> for AccountIdPrefix {
    type Error = AccountIdError;

    /// Tries to convert a byte array in little-endian order to an [`AccountIdPrefix`].
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints of the prefix are not met. See the
    /// [`AccountId`](crate::accounts::AccountId) type documentation for details.
    fn try_from(value: [u8; 8]) -> Result<Self, Self::Error> {
        let element = Felt::try_from(&value[..8])
            .map_err(AccountIdError::AccountIdInvalidPrefixFieldElement)?;
        Self::new(element)
    }
}

impl TryFrom<u64> for AccountIdPrefix {
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

impl TryFrom<Felt> for AccountIdPrefix {
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

impl PartialOrd for AccountIdPrefix {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AccountIdPrefix {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.prefix.as_int().cmp(&other.prefix.as_int())
    }
}

impl fmt::Display for AccountIdPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
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
        Self::SERIALIZED_SIZE
    }
}

impl Deserializable for AccountIdPrefix {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        <[u8; 8]>::read_from(source)?
            .try_into()
            .map_err(|err: AccountIdError| DeserializationError::InvalidValue(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::AccountId;

    #[test]
    fn account_id_prefix_construction() {
        // Use the highest possible input to check if the constructed id is a valid Felt in that
        // scenario.
        // Use the lowest possible input to check whether the constructor produces valid IDs with
        // all-zeroes input.
        for input in [[0xff; 15], [0; 15]] {
            for account_type in [
                AccountType::FungibleFaucet,
                AccountType::NonFungibleFaucet,
                AccountType::RegularAccountImmutableCode,
                AccountType::RegularAccountUpdatableCode,
            ] {
                for storage_mode in [AccountStorageMode::Private, AccountStorageMode::Public] {
                    let id = AccountId::dummy(input, account_type, storage_mode);
                    let prefix = id.prefix();
                    assert_eq!(prefix.account_type(), account_type);
                    assert_eq!(prefix.storage_mode(), storage_mode);
                    assert_eq!(prefix.version(), AccountIdVersion::VERSION_0);

                    // Do a serialization roundtrip to ensure validity.
                    let serialized_prefix = prefix.to_bytes();
                    AccountIdPrefix::read_from_bytes(&serialized_prefix).unwrap();
                    assert_eq!(serialized_prefix.len(), AccountIdPrefix::SERIALIZED_SIZE);
                }
            }
        }
    }
}
