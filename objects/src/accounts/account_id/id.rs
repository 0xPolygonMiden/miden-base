use alloc::string::{String, ToString};
use core::fmt;

use miden_crypto::{merkle::LeafIndex, utils::hex_to_bytes};
use vm_core::{
    utils::{ByteReader, Deserializable, Serializable},
    Felt, Word,
};
use vm_processor::{DeserializationError, Digest};

use crate::{
    accounts::{
        account_id::id_v0, AccountIdAnchor, AccountIdPrefix, AccountIdV0, AccountIdVersion,
        AccountStorageMode, AccountType,
    },
    errors::AccountIdError,
    AccountError, ACCOUNT_TREE_DEPTH,
};

/// The identifier of an [`Account`](crate::accounts::Account).
///
/// This enum is a wrapper around concrete versions of IDs. Refer to the documentation of the
/// concrete versions for details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountId {
    V0(AccountIdV0),
}

impl AccountId {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// The serialized size of an [`AccountId`] in bytes.
    pub const SERIALIZED_SIZE: usize = 15;

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates an [`AccountId`] by hashing the given `seed`, `code_commitment`,
    /// `storage_commitment` and [`AccountIdAnchor::block_hash`] from the `anchor` and using the
    /// resulting first and second element of the hash as the prefix and suffix felts of the ID.
    /// The [`AccountIdAnchor::epoch`] from the `anchor` overwrites part of the suffix.
    ///
    /// Note that the `anchor` must correspond to a valid block in the chain for the ID to be deemed
    /// valid during creation.
    ///
    /// See the documentation of the [`AccountId`] for more details on the generation.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints are not met. See the [type
    /// documentation](AccountId) for details.
    pub fn new(
        seed: Word,
        anchor: AccountIdAnchor,
        version: AccountIdVersion,
        code_commitment: Digest,
        storage_commitment: Digest,
    ) -> Result<Self, AccountIdError> {
        match version {
            AccountIdVersion::Version0 => {
                AccountIdV0::new(seed, anchor, code_commitment, storage_commitment).map(Self::V0)
            },
        }
    }

    /// Creates an [`AccountId`] from the given felts where the felt at index 0 is the prefix
    /// and the felt at index 2 is the suffix.
    ///
    /// # Warning
    ///
    /// Validity of the ID must be ensured by the caller. An invalid ID may lead to panics.
    ///
    /// # Panics
    ///
    /// Panics if the prefix does not contain a known account ID version.
    ///
    /// If debug_assertions are enabled (e.g. in debug mode), this function panics if any of the ID
    /// constraints are not met. See the [type documentation](AccountId) for details.
    pub fn new_unchecked(elements: [Felt; 2]) -> Self {
        // The prefix contains the metadata.
        // If we add more versions in the future, we may need to generalize this.
        match id_v0::extract_version(elements[0].as_int())
            .expect("prefix should contain a valid account ID version")
        {
            AccountIdVersion::Version0 => Self::V0(AccountIdV0::new_unchecked(elements)),
        }
    }

    /// Constructs an [`AccountId`] for testing purposes with the given account type and storage
    /// mode.
    ///
    /// This function does the following:
    /// - Split the given bytes into a `prefix = bytes[0..8]` and `suffix = bytes[8..]` part to be
    ///   used for the prefix and suffix felts, respectively.
    /// - The least significant byte of the prefix is set to the version 0, and the given type and
    ///   storage mode.
    /// - The 32nd most significant bit in the prefix is cleared to ensure it is a valid felt. The
    ///   32nd is chosen as it is the lowest bit that we can clear and still ensure felt validity.
    ///   This leaves the upper 31 bits to be set by the input `bytes` which makes it simpler to
    ///   create test values which more often need specific values for the most significant end of
    ///   the ID.
    /// - In the suffix the anchor epoch is set to 0 and the lower 8 bits are cleared.
    #[cfg(any(feature = "testing", test))]
    pub fn dummy(
        bytes: [u8; 15],
        version: AccountIdVersion,
        account_type: AccountType,
        storage_mode: AccountStorageMode,
    ) -> AccountId {
        match version {
            AccountIdVersion::Version0 => {
                Self::V0(AccountIdV0::dummy(bytes, account_type, storage_mode))
            },
        }
    }

    /// Grinds an account seed until its hash matches the given `account_type`, `storage_mode` and
    /// `version` and returns it as a [`Word`]. The input to the hash function next to the seed are
    /// the `code_commitment`, `storage_commitment` and `anchor_block_hash`.
    ///
    /// The grinding process is started from the given `init_seed` which should be a random seed
    /// generated from a cryptographically secure source.
    pub fn compute_account_seed(
        init_seed: [u8; 32],
        account_type: AccountType,
        storage_mode: AccountStorageMode,
        version: AccountIdVersion,
        code_commitment: Digest,
        storage_commitment: Digest,
        anchor_block_hash: Digest,
    ) -> Result<Word, AccountError> {
        match version {
            AccountIdVersion::Version0 => AccountIdV0::compute_account_seed(
                init_seed,
                account_type,
                storage_mode,
                version,
                code_commitment,
                storage_commitment,
                anchor_block_hash,
            ),
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the type of this account ID.
    pub const fn account_type(&self) -> AccountType {
        match self {
            AccountId::V0(account_id) => account_id.account_type(),
        }
    }

    /// Returns `true` if an account with this ID is a faucet which can issue assets.
    pub fn is_faucet(&self) -> bool {
        self.account_type().is_faucet()
    }

    /// Returns `true` if an account with this ID is a regular account.
    pub fn is_regular_account(&self) -> bool {
        self.account_type().is_regular_account()
    }

    /// Returns the storage mode of this account ID.
    pub fn storage_mode(&self) -> AccountStorageMode {
        match self {
            AccountId::V0(account_id) => account_id.storage_mode(),
        }
    }

    /// Returns `true` if an account with this ID is a public account.
    pub fn is_public(&self) -> bool {
        self.storage_mode() == AccountStorageMode::Public
    }

    /// Returns the version of this account ID.
    pub fn version(&self) -> AccountIdVersion {
        match self {
            AccountId::V0(_) => AccountIdVersion::Version0,
        }
    }

    /// Returns the anchor epoch of this account ID.
    ///
    /// This is the epoch to which this ID is anchored. The hash of this epoch block is used in the
    /// generation of the ID.
    pub fn anchor_epoch(&self) -> u16 {
        match self {
            AccountId::V0(account_id) => account_id.anchor_epoch(),
        }
    }

    /// Creates an [`AccountId`] from a hex string. Assumes the string starts with "0x" and
    /// that the hexadecimal characters are big-endian encoded.
    pub fn from_hex(hex_str: &str) -> Result<Self, AccountIdError> {
        hex_to_bytes(hex_str)
            .map_err(AccountIdError::AccountIdHexParseError)
            .and_then(AccountId::try_from)
    }

    /// Returns a big-endian, hex-encoded string of length 32, including the `0x` prefix. This means
    /// it encodes 15 bytes.
    pub fn to_hex(self) -> String {
        match self {
            AccountId::V0(account_id) => account_id.to_hex(),
        }
    }

    /// Returns the [`AccountIdPrefix`] of this ID.
    ///
    /// The prefix of an account ID is guaranteed to be unique.
    pub fn prefix(&self) -> AccountIdPrefix {
        match self {
            AccountId::V0(account_id) => account_id.prefix(),
        }
    }

    /// Returns the suffix of this ID as a [`Felt`].
    pub const fn suffix(&self) -> Felt {
        match self {
            AccountId::V0(account_id) => account_id.suffix(),
        }
    }
}

// CONVERSIONS FROM ACCOUNT ID
// ================================================================================================

impl From<AccountId> for [Felt; 2] {
    fn from(id: AccountId) -> Self {
        match id {
            AccountId::V0(account_id) => account_id.into(),
        }
    }
}

impl From<AccountId> for [u8; 15] {
    fn from(id: AccountId) -> Self {
        match id {
            AccountId::V0(account_id) => account_id.into(),
        }
    }
}

impl From<AccountId> for u128 {
    fn from(id: AccountId) -> Self {
        match id {
            AccountId::V0(account_id) => account_id.into(),
        }
    }
}

/// Account IDs are used as indexes in the account database, which is a tree of depth 64.
impl From<AccountId> for LeafIndex<ACCOUNT_TREE_DEPTH> {
    fn from(id: AccountId) -> Self {
        match id {
            AccountId::V0(account_id) => account_id.into(),
        }
    }
}

// CONVERSIONS TO ACCOUNT ID
// ================================================================================================

impl From<AccountIdV0> for AccountId {
    fn from(id: AccountIdV0) -> Self {
        Self::V0(id)
    }
}

impl TryFrom<[Felt; 2]> for AccountId {
    type Error = AccountIdError;

    /// Returns an [`AccountId`] instantiated with the provided field elements where `elements[0]`
    /// is taken as the prefix and `elements[1]` is taken as the second element.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints are not met. See the [type
    /// documentation](AccountId) for details.
    fn try_from(elements: [Felt; 2]) -> Result<Self, Self::Error> {
        // The prefix contains the metadata.
        // If we add more versions in the future, we may need to generalize this.
        match id_v0::extract_version(elements[0].as_int())? {
            AccountIdVersion::Version0 => AccountIdV0::try_from(elements).map(Self::V0),
        }
    }
}

impl TryFrom<[u8; 15]> for AccountId {
    type Error = AccountIdError;

    /// Tries to convert a byte array in big-endian order to an [`AccountId`].
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints are not met. See the [type
    /// documentation](AccountId) for details.
    fn try_from(bytes: [u8; 15]) -> Result<Self, Self::Error> {
        // The least significant byte of the ID prefix contains the metadata.
        let metadata_byte = bytes[7];
        // We only have one supported version for now, so we use the extractor from that version.
        // If we add more versions in the future, we may need to generalize this.
        let version = id_v0::extract_version(metadata_byte as u64)?;

        match version {
            AccountIdVersion::Version0 => AccountIdV0::try_from(bytes).map(Self::V0),
        }
    }
}

impl TryFrom<u128> for AccountId {
    type Error = AccountIdError;

    /// Tries to convert a u128 into an [`AccountId`].
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints are not met. See the [type
    /// documentation](AccountId) for details.
    fn try_from(int: u128) -> Result<Self, Self::Error> {
        let mut bytes: [u8; 15] = [0; 15];
        bytes.copy_from_slice(&int.to_be_bytes()[0..15]);

        Self::try_from(bytes)
    }
}

// COMMON TRAIT IMPLS
// ================================================================================================

impl PartialOrd for AccountId {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AccountId {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        u128::from(*self).cmp(&u128::from(*other))
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountId {
    fn write_into<W: miden_crypto::utils::ByteWriter>(&self, target: &mut W) {
        match self {
            AccountId::V0(account_id) => {
                account_id.write_into(target);
            },
        }
    }

    fn get_size_hint(&self) -> usize {
        match self {
            AccountId::V0(account_id) => account_id.get_size_hint(),
        }
    }
}

impl Deserializable for AccountId {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        <[u8; 15]>::read_from(source)?
            .try_into()
            .map_err(|err: AccountIdError| DeserializationError::InvalidValue(err.to_string()))
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::account_id::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
        ACCOUNT_ID_OFF_CHAIN_SENDER, ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
    };

    #[test]
    fn test_account_id_wrapper_conversion_roundtrip() {
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
            let wrapper = AccountId::try_from(account_id).unwrap();
            assert_eq!(
                wrapper,
                AccountId::read_from_bytes(&wrapper.to_bytes()).unwrap(),
                "failed in {idx}"
            );
        }
    }
}
