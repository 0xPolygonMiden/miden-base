mod id_anchor;
pub use id_anchor::AccountIdAnchor;

pub(crate) mod v0;
pub use v0::{AccountIdPrefixV0, AccountIdV0};

mod id_prefix;
pub use id_prefix::AccountIdPrefix;

mod seed;

mod network_id;
pub use network_id::NetworkId;

mod address_type;
pub use address_type::AddressType;

mod account_type;
pub use account_type::AccountType;

mod storage_mode;
pub use storage_mode::AccountStorageMode;

mod id_version;
use alloc::string::{String, ToString};
use core::fmt;

pub use id_version::AccountIdVersion;
use miden_crypto::utils::hex_to_bytes;
use vm_core::{
    Felt, Word,
    utils::{ByteReader, Deserializable, Serializable},
};
use vm_processor::{DeserializationError, Digest};

use crate::{AccountError, errors::AccountIdError};

/// The identifier of an [`Account`](crate::account::Account).
///
/// This enum is a wrapper around concrete versions of IDs. The following documents version 0.
///
/// # Layout
///
/// An `AccountId` consists of two field elements and is layed out as follows:
///
/// ```text
/// 1st felt: [random (56 bits) | storage mode (2 bits) | type (2 bits) | version (4 bits)]
/// 2nd felt: [anchor_epoch (16 bits) | random (40 bits) | 8 zero bits]
/// ```
///
/// # Generation
///
/// An `AccountId` is a commitment to a user-generated seed, the code and storage of an account and
/// to a certain hash of an epoch block of the blockchain. An id is generated by picking an epoch
/// block as an anchor - which is why it is also referred to as the anchor block - and creating the
/// account's initial storage and code. Then a random seed is picked and the hash of `(SEED,
/// CODE_COMMITMENT, STORAGE_COMMITMENT, ANCHOR_BLOCK_COMMITMENT)` is computed. If the hash's first
/// element has the desired storage mode, account type and version, the computation part of the ID
/// generation is done. If not, another random seed is picked and the process is repeated. The first
/// felt of the ID is then the first element of the hash.
///
/// The suffix of the ID is the second element of the hash. Its upper 16 bits are overwritten
/// with the epoch in which the ID is anchored and the lower 8 bits are zeroed. Thus, the prefix
/// of the ID must derive exactly from the hash, while only part of the suffix is derived from
/// the hash.
///
/// # Constraints
///
/// Constructors will return an error if:
///
/// - The prefix contains account ID metadata (storage mode, type or version) that does not match
///   any of the known values.
/// - The anchor epoch in the suffix is equal to [`u16::MAX`].
/// - The lower 8 bits of the suffix are not zero, although [`AccountId::new`] ensures this is the
///   case rather than return an error.
///
/// # Design Rationale
///
/// The rationale behind the above layout is as follows.
///
/// - The prefix is the output of a hash function so it will be a valid field element without
///   requiring additional constraints.
/// - The version is placed at a static offset such that future ID versions which may change the
///   number of type or storage mode bits will not cause the version to be at a different offset.
///   This is important so that a parser can always reliably read the version and then parse the
///   remainder of the ID depending on the version. Having only 4 bits for the version is a trade
///   off between future proofing to be able to introduce more versions and the version requiring
///   Proof of Work as part of the ID generation.
/// - The version, type and storage mode are part of the prefix which is included in the
///   representation of a non-fungible asset. The prefix alone is enough to determine all of these
///   properties about the ID.
///     - The anchor epoch is not important beyond the creation process, so placing it in the second
///       felt is fine. Moreover, all properties of the prefix must be derived from the seed, so
///       they add to the proof of work difficulty. Adding 16 bits of PoW for the epoch would be
///       significant.
/// - The anchor epoch is placed at the most significant end of the suffix. Its value must be less
///   than [`u16::MAX`] so that at least one of the upper 16 bits is always zero. This ensures that
///   the entire suffix is valid even if the remaining bits of the felt are one.
/// - The lower 8 bits of the suffix may be overwritten when the ID is encoded in other layouts such
///   as the [`NoteMetadata`](crate::note::NoteMetadata). In such cases, it can happen that all bits
///   of the encoded suffix would be one, so having the epoch constraint is important.
/// - The ID is dependent on the hash of an epoch block. This is a block whose number is a multiple
///   of 2^[`BlockNumber::EPOCH_LENGTH_EXPONENT`][epoch_len_exp], e.g. `0`, `65536`, `131072`, ...
///   These are the first blocks of epoch 0, 1, 2, ... We call this dependence _anchoring_ because
///   the ID is anchored to that epoch block's commitment. Anchoring makes it practically impossible
///   for an attacker to construct a rainbow table of account IDs whose epoch is X, if the block for
///   epoch X has not been constructed yet because its hash is then unknown. Therefore, picking a
///   recent anchor block when generating a new ID makes it extremely unlikely that an attacker can
///   highjack this ID because the hash of that block has only been known for a short period of
///   time.
///     - An ID highjack refers to an attack where a user generates an ID and lets someone else send
///       assets to it. At this point the user has not registered the ID on-chain yet, likely
///       because they need the funds in the asset to pay for their first transaction where the
///       account would be registered. Until the ID is registered on chain, an attacker with a
///       rainbow table who happens to have a seed, code and storage commitment combination that
///       hashes to the user's ID can claim the assets sent to the user's ID. Adding the anchor
///       block commitment to the ID generation process makes this attack practically impossible.
///
/// [epoch_len_exp]: crate::block::BlockNumber::EPOCH_LENGTH_EXPONENT
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
    /// `storage_commitment` and [`AccountIdAnchor::block_commitment`] from the `anchor` and using
    /// the resulting first and second element of the hash as the prefix and suffix felts of the
    /// ID.
    ///
    /// The [`AccountIdAnchor::epoch`] from the `anchor` overwrites part of the suffix.
    ///
    /// Note that the `anchor` must correspond to a valid block in the chain for the ID to be deemed
    /// valid during creation.
    ///
    /// See the documentation of the [`AccountId`] for more details on the generation.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints are not met. See the [constraints
    /// documentation](AccountId#constraints) for details.
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
    /// constraints are not met. See the [constraints documentation](AccountId#constraints) for
    /// details.
    pub fn new_unchecked(elements: [Felt; 2]) -> Self {
        // The prefix contains the metadata.
        // If we add more versions in the future, we may need to generalize this.
        match v0::extract_version(elements[0].as_int())
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
    /// the `code_commitment`, `storage_commitment` and `anchor_block_commitment`.
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
        anchor_block_commitment: Digest,
    ) -> Result<Word, AccountError> {
        match version {
            AccountIdVersion::Version0 => AccountIdV0::compute_account_seed(
                init_seed,
                account_type,
                storage_mode,
                version,
                code_commitment,
                storage_commitment,
                anchor_block_commitment,
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

    /// Encodes the [`AccountId`] into a [bech32](https://github.com/bitcoin/bips/blob/master/bip-0173.mediawiki) string.
    ///
    /// # Encoding
    ///
    /// The encoding of an account ID into bech32 is done as follows:
    /// - Convert the account ID into its `[u8; 15]` data format.
    /// - Insert the address type [`AddressType::AccountId`] byte at index 0, shifting all other
    ///   elements to the right.
    /// - Choose an HRP, defined as a [`NetworkId`], for example [`NetworkId::Mainnet`] whose string
    ///   representation is `mm`.
    /// - Encode the resulting HRP together with the data into a bech32 string using the
    ///   [`bech32::Bech32m`] checksum algorithm.
    ///
    /// This is an example of an account ID in hex and bech32 representations:
    ///
    /// ```text
    /// hex:    0x140fa04a1e61fc100000126ef8f1d6
    /// bech32: mm1qq2qlgz2reslcyqqqqfxa7836chrjcvk
    /// ```
    ///
    /// ## Rationale
    ///
    /// Having the address type at the very beginning is so that it can be decoded to detect the
    /// type of the address without having to decode the entire data. Moreover, choosing the
    /// address type as a multiple of 8 means the first character of the bech32 string after the
    /// `1` separator will be different for every address type. This makes the type of the address
    /// conveniently human-readable.
    ///
    /// The only allowed checksum algorithm is [`Bech32m`](bech32::Bech32m) due to being the best
    /// available checksum algorithm with no known weaknesses (unlike [`Bech32`](bech32::Bech32)).
    /// No checksum is also not allowed since the intended use of bech32 is to have error
    /// detection capabilities.
    pub fn to_bech32(&self, network_id: NetworkId) -> String {
        match self {
            AccountId::V0(account_id_v0) => account_id_v0.to_bech32(network_id),
        }
    }

    /// Decodes a [bech32](https://github.com/bitcoin/bips/blob/master/bip-0173.mediawiki) string into an [`AccountId`].
    ///
    /// See [`AccountId::to_bech32`] for details on the format. The procedure for decoding the
    /// bech32 data into the ID consists of the inverse operations of encoding.
    pub fn from_bech32(bech32_string: &str) -> Result<(NetworkId, Self), AccountIdError> {
        AccountIdV0::from_bech32(bech32_string)
            .map(|(network_id, account_id)| (network_id, AccountId::V0(account_id)))
    }

    /// Returns the [`AccountIdPrefix`] of this ID.
    ///
    /// The prefix of an account ID is guaranteed to be unique.
    pub fn prefix(&self) -> AccountIdPrefix {
        match self {
            AccountId::V0(account_id) => AccountIdPrefix::V0(account_id.prefix()),
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
    /// is taken as the prefix and `elements[1]` is taken as the suffix.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints are not met. See the [constraints
    /// documentation](AccountId#constraints) for details.
    fn try_from(elements: [Felt; 2]) -> Result<Self, Self::Error> {
        // The prefix contains the metadata.
        // If we add more versions in the future, we may need to generalize this.
        match v0::extract_version(elements[0].as_int())? {
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
    /// Returns an error if any of the ID constraints are not met. See the [constraints
    /// documentation](AccountId#constraints) for details.
    fn try_from(bytes: [u8; 15]) -> Result<Self, Self::Error> {
        // The least significant byte of the ID prefix contains the metadata.
        let metadata_byte = bytes[7];
        // We only have one supported version for now, so we use the extractor from that version.
        // If we add more versions in the future, we may need to generalize this.
        let version = v0::extract_version(metadata_byte as u64)?;

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
    /// Returns an error if any of the ID constraints are not met. See the [constraints
    /// documentation](AccountId#constraints) for details.
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
    use assert_matches::assert_matches;
    use bech32::{Bech32, Bech32m, Hrp, NoChecksum};

    use super::*;
    use crate::{
        account::account_id::{
            address_type::AddressType,
            v0::{extract_storage_mode, extract_type, extract_version},
        },
        errors::Bech32Error,
        testing::account_id::{
            ACCOUNT_ID_PRIVATE_NON_FUNGIBLE_FAUCET, ACCOUNT_ID_PRIVATE_SENDER,
            ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET, ACCOUNT_ID_REGULAR_PRIVATE_ACCOUNT_UPDATABLE_CODE,
            ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE,
        },
    };

    #[test]
    fn test_account_id_wrapper_conversion_roundtrip() {
        for (idx, account_id) in [
            ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE,
            ACCOUNT_ID_REGULAR_PRIVATE_ACCOUNT_UPDATABLE_CODE,
            ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET,
            ACCOUNT_ID_PRIVATE_NON_FUNGIBLE_FAUCET,
            ACCOUNT_ID_PRIVATE_SENDER,
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

    #[test]
    fn bech32_encode_decode_roundtrip() {
        // We use this to check that encoding does not panic even when using the longest possible
        // HRP.
        let longest_possible_hrp =
            "01234567890123456789012345678901234567890123456789012345678901234567890123456789012";
        assert_eq!(longest_possible_hrp.len(), 83);

        for network_id in [
            NetworkId::Mainnet,
            NetworkId::Custom(Hrp::parse("custom").unwrap()),
            NetworkId::Custom(Hrp::parse(longest_possible_hrp).unwrap()),
        ] {
            for (idx, account_id) in [
                ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE,
                ACCOUNT_ID_REGULAR_PRIVATE_ACCOUNT_UPDATABLE_CODE,
                ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET,
                ACCOUNT_ID_PRIVATE_NON_FUNGIBLE_FAUCET,
                ACCOUNT_ID_PRIVATE_SENDER,
            ]
            .into_iter()
            .enumerate()
            {
                let account_id = AccountId::try_from(account_id).unwrap();

                let bech32_string = account_id.to_bech32(network_id);
                let (decoded_network_id, decoded_account_id) =
                    AccountId::from_bech32(&bech32_string).unwrap();

                assert_eq!(network_id, decoded_network_id, "network id failed in {idx}");
                assert_eq!(account_id, decoded_account_id, "account id failed in {idx}");

                let (_, data) = bech32::decode(&bech32_string).unwrap();

                // Raw bech32 data should contain the address type as the first byte.
                assert_eq!(data[0], AddressType::AccountId as u8);

                // Raw bech32 data should contain the metadata byte at index 8.
                assert_eq!(extract_version(data[8] as u64).unwrap(), account_id.version());
                assert_eq!(extract_type(data[8] as u64), account_id.account_type());
                assert_eq!(
                    extract_storage_mode(data[8] as u64).unwrap(),
                    account_id.storage_mode()
                );
            }
        }
    }

    #[test]
    fn bech32_invalid_checksum() {
        let network_id = NetworkId::Mainnet;
        let account_id = AccountId::try_from(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET).unwrap();

        let bech32_string = account_id.to_bech32(network_id);
        let mut invalid_bech32_1 = bech32_string.clone();
        invalid_bech32_1.remove(0);
        let mut invalid_bech32_2 = bech32_string.clone();
        invalid_bech32_2.remove(7);

        let error = AccountId::from_bech32(&invalid_bech32_1).unwrap_err();
        assert_matches!(error, AccountIdError::Bech32DecodeError(Bech32Error::DecodeError(_)));

        let error = AccountId::from_bech32(&invalid_bech32_2).unwrap_err();
        assert_matches!(error, AccountIdError::Bech32DecodeError(Bech32Error::DecodeError(_)));
    }

    #[test]
    fn bech32_invalid_address_type() {
        let account_id = AccountId::try_from(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET).unwrap();
        let mut id_bytes = account_id.to_bytes();

        // Set invalid address type.
        id_bytes.insert(0, 16);

        let invalid_bech32 =
            bech32::encode::<Bech32m>(NetworkId::Mainnet.into_hrp(), &id_bytes).unwrap();

        let error = AccountId::from_bech32(&invalid_bech32).unwrap_err();
        assert_matches!(
            error,
            AccountIdError::Bech32DecodeError(Bech32Error::UnknownAddressType(16))
        );
    }

    #[test]
    fn bech32_invalid_other_checksum() {
        let account_id = AccountId::try_from(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET).unwrap();
        let mut id_bytes = account_id.to_bytes();
        id_bytes.insert(0, AddressType::AccountId as u8);

        // Use Bech32 instead of Bech32m which is disallowed.
        let invalid_bech32_regular =
            bech32::encode::<Bech32>(NetworkId::Mainnet.into_hrp(), &id_bytes).unwrap();
        let error = AccountId::from_bech32(&invalid_bech32_regular).unwrap_err();
        assert_matches!(error, AccountIdError::Bech32DecodeError(Bech32Error::DecodeError(_)));

        // Use no checksum instead of Bech32m which is disallowed.
        let invalid_bech32_no_checksum =
            bech32::encode::<NoChecksum>(NetworkId::Mainnet.into_hrp(), &id_bytes).unwrap();
        let error = AccountId::from_bech32(&invalid_bech32_no_checksum).unwrap_err();
        assert_matches!(error, AccountIdError::Bech32DecodeError(Bech32Error::DecodeError(_)));
    }

    #[test]
    fn bech32_invalid_length() {
        let account_id = AccountId::try_from(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET).unwrap();
        let mut id_bytes = account_id.to_bytes();
        id_bytes.insert(0, AddressType::AccountId as u8);
        // Add one byte to make the length invalid.
        id_bytes.push(5);

        let invalid_bech32 =
            bech32::encode::<Bech32m>(NetworkId::Mainnet.into_hrp(), &id_bytes).unwrap();

        let error = AccountId::from_bech32(&invalid_bech32).unwrap_err();
        assert_matches!(
            error,
            AccountIdError::Bech32DecodeError(Bech32Error::InvalidDataLength { .. })
        );
    }
}
