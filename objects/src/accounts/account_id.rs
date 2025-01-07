use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{fmt, str::FromStr};

use miden_crypto::{merkle::LeafIndex, utils::hex_to_bytes};
use vm_core::{
    utils::{ByteReader, Deserializable, Serializable},
    Felt, Word,
};
use vm_processor::{DeserializationError, Digest};

use super::Hasher;
use crate::{
    accounts::{AccountIdAnchor, AccountIdPrefix},
    AccountError, ACCOUNT_TREE_DEPTH,
};

// ACCOUNT TYPE
// ================================================================================================

const FUNGIBLE_FAUCET: u64 = 0b10;
const NON_FUNGIBLE_FAUCET: u64 = 0b11;
const REGULAR_ACCOUNT_IMMUTABLE_CODE: u64 = 0b00;
const REGULAR_ACCOUNT_UPDATABLE_CODE: u64 = 0b01;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u64)]
pub enum AccountType {
    FungibleFaucet = FUNGIBLE_FAUCET,
    NonFungibleFaucet = NON_FUNGIBLE_FAUCET,
    RegularAccountImmutableCode = REGULAR_ACCOUNT_IMMUTABLE_CODE,
    RegularAccountUpdatableCode = REGULAR_ACCOUNT_UPDATABLE_CODE,
}

impl AccountType {
    /// Returns `true` if the account is a faucet.
    pub fn is_faucet(&self) -> bool {
        matches!(self, Self::FungibleFaucet | Self::NonFungibleFaucet)
    }

    /// Returns `true` if the account is a regular account.
    pub fn is_regular_account(&self) -> bool {
        matches!(self, Self::RegularAccountImmutableCode | Self::RegularAccountUpdatableCode)
    }
}

impl From<AccountId> for AccountType {
    fn from(id: AccountId) -> Self {
        id.account_type()
    }
}

impl From<AccountIdPrefix> for AccountType {
    fn from(id_prefix: AccountIdPrefix) -> Self {
        id_prefix.account_type()
    }
}

impl Serializable for AccountType {
    fn write_into<W: vm_core::utils::ByteWriter>(&self, target: &mut W) {
        target.write_u64(*self as u64);
    }
}

impl Deserializable for AccountType {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let num: u64 = source.read()?;
        match num {
            FUNGIBLE_FAUCET => Ok(AccountType::FungibleFaucet),
            NON_FUNGIBLE_FAUCET => Ok(AccountType::NonFungibleFaucet),
            REGULAR_ACCOUNT_IMMUTABLE_CODE => Ok(AccountType::RegularAccountImmutableCode),
            REGULAR_ACCOUNT_UPDATABLE_CODE => Ok(AccountType::RegularAccountUpdatableCode),
            _ => Err(DeserializationError::InvalidValue(format!("invalid account type: {num}"))),
        }
    }
}

// SERIALIZATION
// ================================================================================================

#[cfg(feature = "std")]
impl serde::Serialize for AccountType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            AccountType::FungibleFaucet => "FungibleFaucet",
            AccountType::NonFungibleFaucet => "NonFungibleFaucet",
            AccountType::RegularAccountImmutableCode => "RegularAccountImmutableCode",
            AccountType::RegularAccountUpdatableCode => "RegularAccountUpdatableCode",
        };
        serializer.serialize_str(s)
    }
}

#[cfg(feature = "std")]
impl<'de> serde::Deserialize<'de> for AccountType {
    fn deserialize<D>(deserializer: D) -> Result<AccountType, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let s: String = serde::Deserialize::deserialize(deserializer)?;

        match s.as_str() {
            "FungibleFaucet" => Ok(AccountType::FungibleFaucet),
            "NonFungibleFaucet" => Ok(AccountType::NonFungibleFaucet),
            "RegularAccountImmutableCode" => Ok(AccountType::RegularAccountImmutableCode),
            "RegularAccountUpdatableCode" => Ok(AccountType::RegularAccountUpdatableCode),
            other => Err(D::Error::invalid_value(
                serde::de::Unexpected::Str(other),
                &"a valid account type (\"FungibleFaucet\", \"NonFungibleFaucet\", \"RegularAccountImmutableCode\", or \"RegularAccountUpdatableCode\")",
            )),
        }
    }
}

// ACCOUNT STORAGE MODE
// ================================================================================================

const PUBLIC: u64 = 0b00;
const PRIVATE: u64 = 0b10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum AccountStorageMode {
    Public = PUBLIC,
    Private = PRIVATE,
}

impl fmt::Display for AccountStorageMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountStorageMode::Public => write!(f, "public"),
            AccountStorageMode::Private => write!(f, "private"),
        }
    }
}

impl TryFrom<&str> for AccountStorageMode {
    type Error = AccountError;

    fn try_from(value: &str) -> Result<Self, AccountError> {
        match value.to_lowercase().as_str() {
            "public" => Ok(AccountStorageMode::Public),
            "private" => Ok(AccountStorageMode::Private),
            _ => Err(AccountError::InvalidAccountStorageMode(value.into())),
        }
    }
}

impl TryFrom<String> for AccountStorageMode {
    type Error = AccountError;

    fn try_from(value: String) -> Result<Self, AccountError> {
        AccountStorageMode::from_str(&value)
    }
}

impl FromStr for AccountStorageMode {
    type Err = AccountError;

    fn from_str(input: &str) -> Result<AccountStorageMode, AccountError> {
        AccountStorageMode::try_from(input)
    }
}

impl From<AccountId> for AccountStorageMode {
    fn from(id: AccountId) -> Self {
        id.storage_mode()
    }
}

impl From<AccountIdPrefix> for AccountStorageMode {
    fn from(id_prefix: AccountIdPrefix) -> Self {
        id_prefix.storage_mode()
    }
}

// ACCOUNT ID VERSION
// ================================================================================================

/// The version of an [`AccountId`].
///
/// Each version has a public associated constant, e.g. [`AccountIdVersion::VERSION_0`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountIdVersion(u8);

impl AccountIdVersion {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    const VERSION_0_NUMBER: u8 = 0;
    /// Version 0 of the [`AccountId`].
    pub const VERSION_0: AccountIdVersion = AccountIdVersion(Self::VERSION_0_NUMBER);

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the version number.
    pub const fn as_u8(&self) -> u8 {
        self.0
    }
}

impl From<AccountIdVersion> for u8 {
    fn from(value: AccountIdVersion) -> Self {
        value.as_u8()
    }
}

impl From<AccountId> for AccountIdVersion {
    fn from(id: AccountId) -> Self {
        id.version()
    }
}

impl From<AccountIdPrefix> for AccountIdVersion {
    fn from(id_prefix: AccountIdPrefix) -> Self {
        id_prefix.version()
    }
}

// ACCOUNT ID
// ================================================================================================

/// The identifier of an [`Account`](crate::accounts::Account).
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
/// account's initial storage and code. Then a random seed is picked and the hash of (SEED,
/// CODE_COMMITMENT, STORAGE_COMMITMENT, ANCHOR_BLOCK_HASH) is computed. If the hash's first element
/// has the desired storage mode, account type, version and the high bit set to zero, the
/// computation part of the ID generation is done. If not, another random seed is picked and the
/// process is repeated. The first felt of the ID is then the first element of the hash.
///
/// The second felt of the ID is the second element of the hash. Its upper 16 bits are overwritten
/// with the epoch in which the ID is anchored and the lower 8 bits are zeroed. Thus, the first felt
/// of the ID must derive exactly from the hash, while only part of the second felt is derived from
/// the hash.
///
/// # Constraints
///
/// Constructors will return an error if:
///
/// - The first felt contains account ID metadata (storage mode, type or version) that does not
///   match any of the known values.
/// - The anchor epoch in the second felt is equal to [`u16::MAX`].
/// - The lower 8 bits of the second felt are not zero, although [`AccountId::new`] ensures this is
///   the case rather than return an error.
///
/// # Design Rationale
///
/// The rationale behind the above layout is as follows.
///
/// - The first felt is the output of a hash function so it will be a valid field element without
///   requiring additional constraints.
/// - The version is placed at a static offset such that future ID versions which may change the
///   number of type or storage mode bits will not cause the version to be at a different offset.
///   This is important so that a parser can always reliably read the version and then parse the
///   remainder of the ID depending on the version. Having only 4 bits for the version is a trade
///   off between future proofing to be able to introduce more versions and the version requiring
///   Proof of Work as part of the ID generation.
/// - The version, type and storage mode are part of the first felt which is included in the
///   representation of a non-fungible asset. The first felt alone is enough to determine all of
///   these properties about the ID.
///     - The anchor epoch is not important beyond the creation process, so placing it in the second
///       felt is fine. Moreover, all properties of the first felt must be derived from the seed, so
///       they add to the proof of work difficulty. Adding 16 bits of PoW for the epoch would be
///       significant.
/// - The anchor epoch is placed at the most significant end of the second felt. Its value must be
///   less than [`u16::MAX`] so that at least one of the upper 16 bits is always zero. This ensures
///   that the entire second felt is valid even if the remaining bits of the felt are one.
/// - The lower 8 bits of the second felt may be overwritten when the ID is encoded in other layouts
///   such as the [`NoteMetadata`](crate::notes::NoteMetadata). In such cases, it can happen that
///   all bits of the encoded second felt would be one, so having the epoch constraint is important.
/// - The ID is dependent on the hash of an epoch block. This is a block whose number is a multiple
///   of 2^[`BlockHeader::EPOCH_LENGTH_EXPONENT`][epoch_len_exp], e.g. `0`, `65536`, `131072`, ...
///   These are the first blocks of epoch 0, 1, 2, ... We call this dependence _anchoring_ because
///   the ID is anchored to that epoch block's hash. Anchoring makes it practically impossible for
///   an attacker to construct a rainbow table of account IDs whose epoch is X, if the block for
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
///       block hash to ID generation process makes this attack practically impossible.
///
/// [epoch_len_exp]: crate::block::BlockHeader::EPOCH_LENGTH_EXPONENT
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AccountId {
    first_felt: Felt,
    second_felt: Felt,
}

impl AccountId {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// The serialized size of an [`AccountId`] in bytes.
    pub const SERIALIZED_SIZE: usize = 15;

    /// The lower two bits of the second least significant nibble determine the account type.
    pub(crate) const TYPE_SHIFT: u64 = 4;
    pub(crate) const TYPE_MASK: u64 = 0b11 << Self::TYPE_SHIFT;

    /// The least significant nibble determines the account version.
    const VERSION_MASK: u64 = 0b1111;

    const ANCHOR_EPOCH_SHIFT: u64 = 48;
    const ANCHOR_EPOCH_MASK: u64 = 0xffff << Self::ANCHOR_EPOCH_SHIFT;

    /// The higher two bits of the second least significant nibble determine the account storage
    /// mode.
    pub(crate) const STORAGE_MODE_SHIFT: u64 = 6;
    pub(crate) const STORAGE_MODE_MASK: u64 = 0b11 << Self::STORAGE_MODE_SHIFT;

    pub(crate) const IS_FAUCET_MASK: u64 = 0b10 << Self::TYPE_SHIFT;

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates an [`AccountId`] by hashing the given `seed`, `code_commitment`,
    /// `storage_commitment` and [`AccountIdAnchor::block_hash`] from the `anchor` and using the
    /// resulting first and second element of the hash as the first and second felt of the ID.
    /// The [`AccountIdAnchor::epoch`] from the `anchor` overwrites part of the second felt.
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
        code_commitment: Digest,
        storage_commitment: Digest,
    ) -> Result<Self, AccountError> {
        let seed_digest =
            compute_digest(seed, code_commitment, storage_commitment, anchor.block_hash());

        let mut felts: [Felt; 2] = seed_digest.as_elements()[0..2]
            .try_into()
            .expect("we should have sliced off 2 elements");

        felts[1] = shape_second_felt(felts[1], anchor.epoch());

        // This will validate that the anchor_epoch we have just written is not u16::MAX.
        account_id_from_felts(felts)
    }

    /// Creates an [`AccountId`] from the given felts where the felt at index 0 is the first felt
    /// and the felt at index 2 is the second felt.
    ///
    /// # Warning
    ///
    /// Validity of the ID must be ensured by the caller. An invalid ID may lead to panics.
    ///
    /// # Panics
    ///
    /// If debug_assertions are enabled (e.g. in debug mode), this function panics if any of the ID
    /// constraints are not met. See the [type documentation](AccountId) for details.
    pub fn new_unchecked(elements: [Felt; 2]) -> Self {
        let first_felt = elements[0];
        let second_felt = elements[1];

        // Panic on invalid felts in debug mode.
        if cfg!(debug_assertions) {
            validate_first_felt(first_felt)
                .expect("AccountId::new_unchecked called with invalid first felt");
            validate_second_felt(second_felt)
                .expect("AccountId::new_unchecked called with invalid first felt");
        }

        Self { first_felt, second_felt }
    }

    /// Constructs an [`AccountId`] for testing purposes with the given account type and storage
    /// mode.
    ///
    /// This function does the following:
    /// - Split the given bytes into a `first_felt = bytes[0..8]` and `second_felt = bytes[8..]`
    ///   part to be used for the first and second felt, respectively.
    /// - The least significant byte of the first felt is set to the version 0, and the given type
    ///   and storage mode.
    /// - The 32nd most significant bit in the first felt is cleared to ensure it is a valid felt.
    ///   The 32nd is chosen as it is the lowest bit that we can clear and still ensure felt
    ///   validity. This leaves the upper 31 bits to be set by the input `bytes` which makes it
    ///   simpler to create test values which more often need specific values for the most
    ///   significant end of the ID.
    /// - In the second felt the anchor epoch is set to 0 and the lower 8 bits are cleared.
    #[cfg(any(feature = "testing", test))]
    pub fn new_dummy(
        mut bytes: [u8; 15],
        account_type: AccountType,
        storage_mode: AccountStorageMode,
    ) -> AccountId {
        let version = AccountIdVersion::VERSION_0_NUMBER;
        let low_nibble = (storage_mode as u8) << Self::STORAGE_MODE_SHIFT
            | (account_type as u8) << Self::TYPE_SHIFT
            | version;

        // Set least significant byte.
        bytes[7] = low_nibble;

        // Clear the 32nd most significant bit.
        bytes[3] &= 0b1111_1110;

        let first_felt_bytes =
            bytes[0..8].try_into().expect("we should have sliced off exactly 8 bytes");
        let first_felt = Felt::try_from(u64::from_be_bytes(first_felt_bytes))
            .expect("should be a valid felt due to the most significant bit being zero");

        let mut second_felt_bytes = [0; 8];
        // Overwrite first 7 bytes, leaving the 8th byte 0 (which will be cleared by
        // shape_second_felt anyway).
        second_felt_bytes[..7].copy_from_slice(&bytes[8..]);
        // If the value is too large modular reduction is performed, which is fine here.
        let mut second_felt = Felt::new(u64::from_be_bytes(second_felt_bytes));

        second_felt = shape_second_felt(second_felt, 0);

        let account_id = account_id_from_felts([first_felt, second_felt])
            .expect("we should have shaped the felts to produce a valid id");

        debug_assert_eq!(account_id.account_type(), account_type);
        debug_assert_eq!(account_id.storage_mode(), storage_mode);

        account_id
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
        crate::accounts::seed::compute_account_seed(
            init_seed,
            account_type,
            storage_mode,
            version,
            code_commitment,
            storage_commitment,
            anchor_block_hash,
        )
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the type of this account ID.
    pub const fn account_type(&self) -> AccountType {
        extract_type(self.first_felt().as_int())
    }

    /// Returns true if an account with this ID is a faucet which can issue assets.
    pub fn is_faucet(&self) -> bool {
        self.account_type().is_faucet()
    }

    /// Returns true if an account with this ID is a regular account.
    pub fn is_regular_account(&self) -> bool {
        self.account_type().is_regular_account()
    }

    /// Returns the storage mode of this account ID.
    pub fn storage_mode(&self) -> AccountStorageMode {
        extract_storage_mode(self.first_felt().as_int())
            .expect("account id should have been constructed with a valid storage mode")
    }

    /// Returns true if an account with this ID is a public account.
    pub fn is_public(&self) -> bool {
        self.storage_mode() == AccountStorageMode::Public
    }

    /// Returns the version of this account ID.
    pub fn version(&self) -> AccountIdVersion {
        extract_version(self.first_felt().as_int())
            .expect("account id should have been constructed with a valid version")
    }

    /// Returns the anchor epoch of this account ID.
    ///
    /// This is the epoch to which this ID is anchored. The hash of this epoch block is used in the
    /// generation of the ID.
    pub fn anchor_epoch(&self) -> u16 {
        extract_anchor_epoch(self.second_felt().as_int())
    }

    /// Creates an [`AccountId`] from a hex string. Assumes the string starts with "0x" and
    /// that the hexadecimal characters are big-endian encoded.
    pub fn from_hex(hex_str: &str) -> Result<AccountId, AccountError> {
        hex_to_bytes(hex_str).map_err(AccountError::AccountIdHexParseError).and_then(
            |mut bytes: [u8; 15]| {
                // TryFrom<[u8; 15]> expects [first_felt, second_felt] in little-endian order, so we
                // need to convert the bytes representation from big endian to little endian by
                // reversing each felt. The first felt has 8 and the second felt has
                // 7 bytes.
                bytes[0..8].reverse();
                bytes[8..15].reverse();

                AccountId::try_from(bytes)
            },
        )
    }

    /// Returns a big-endian, hex-encoded string of length 32, including the `0x` prefix, so it
    /// encodes 15 bytes.
    pub fn to_hex(&self) -> String {
        // We need to pad the second felt with 16 zeroes so it produces a correctly padded 8 byte
        // big-endian hex string. Only then can we cut off the last zero byte by truncating. We
        // cannot use `:014x` padding.
        let mut hex_string =
            format!("0x{:016x}{:016x}", self.first_felt().as_int(), self.second_felt().as_int());
        hex_string.truncate(32);
        hex_string
    }

    /// Returns the [`AccountIdPrefix`] of this ID which is equivalent to the first felt.
    pub fn prefix(&self) -> AccountIdPrefix {
        // SAFETY: We only construct accounts with valid first felts, so we don't have to validate
        // it again.
        AccountIdPrefix::new_unchecked(self.first_felt)
    }

    /// Returns the first felt of this ID.
    pub const fn first_felt(&self) -> Felt {
        self.first_felt
    }

    /// Returns the second felt of this ID.
    pub const fn second_felt(&self) -> Felt {
        self.second_felt
    }
}

// CONVERSIONS FROM ACCOUNT ID
// ================================================================================================

impl From<AccountId> for [Felt; 2] {
    fn from(id: AccountId) -> Self {
        [id.first_felt, id.second_felt]
    }
}

impl From<AccountId> for [u8; 15] {
    fn from(id: AccountId) -> Self {
        let mut result = [0_u8; 15];
        result[..8].copy_from_slice(&id.first_felt().as_int().to_le_bytes());
        // The last byte of the second felt is always zero, and in little endian this is the first
        // byte, so we skip it here.
        result[8..].copy_from_slice(&id.second_felt().as_int().to_le_bytes()[1..8]);
        result
    }
}

impl From<AccountId> for u128 {
    fn from(id: AccountId) -> Self {
        let mut le_bytes = [0_u8; 16];
        le_bytes[..8].copy_from_slice(&id.second_felt().as_int().to_le_bytes());
        le_bytes[8..].copy_from_slice(&id.first_felt().as_int().to_le_bytes());
        u128::from_le_bytes(le_bytes)
    }
}

/// Account IDs are used as indexes in the account database, which is a tree of depth 64.
impl From<AccountId> for LeafIndex<ACCOUNT_TREE_DEPTH> {
    fn from(id: AccountId) -> Self {
        LeafIndex::new_max_depth(id.first_felt().as_int())
    }
}

// CONVERSIONS TO ACCOUNT ID
// ================================================================================================

impl TryFrom<[Felt; 2]> for AccountId {
    type Error = AccountError;

    /// Returns an [`AccountId`] instantiated with the provided field elements where `elements[0]`
    /// is taken as the first felt and `elements[1]` is taken as the second element.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints are not met. See the [type
    /// documentation](AccountId) for details.
    fn try_from(elements: [Felt; 2]) -> Result<Self, Self::Error> {
        account_id_from_felts(elements)
    }
}

impl TryFrom<[u8; 15]> for AccountId {
    type Error = AccountError;

    /// Tries to convert a byte array in little-endian order to an [`AccountId`].
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints are not met. See the [type
    /// documentation](AccountId) for details.
    fn try_from(bytes: [u8; 15]) -> Result<Self, Self::Error> {
        // This slice has 8 bytes.
        let first_felt_slice = &bytes[..8];
        // This slice has 7 bytes, since the 8th byte will always be zero.
        let second_felt_slice = &bytes[8..15];

        // The byte order is little-endian order, so prepending a 0 sets the least significant byte.
        let mut second_felt_bytes = [0; 8];
        second_felt_bytes[1..8].copy_from_slice(second_felt_slice);

        let first_felt =
            Felt::try_from(first_felt_slice).map_err(AccountError::AccountIdInvalidFieldElement)?;

        let second_felt = Felt::try_from(second_felt_bytes.as_slice())
            .map_err(AccountError::AccountIdInvalidFieldElement)?;

        Self::try_from([first_felt, second_felt])
    }
}

impl TryFrom<u128> for AccountId {
    type Error = AccountError;

    /// Tries to convert a u128 into an [`AccountId`].
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints are not met. See the [type
    /// documentation](AccountId) for details.
    fn try_from(int: u128) -> Result<Self, Self::Error> {
        let little_endian_bytes = int.to_le_bytes();
        let mut bytes: [u8; 15] = [0; 15];

        // Swap the positions of the Felts to match what the TryFrom<[u8; 15]> impl expects.
        // This copies the first felt's 8 bytes.
        bytes[..8].copy_from_slice(&little_endian_bytes[8..]);
        // This copies the second felt's 7 bytes. The least significant byte is zero and is
        // therefore skipped.
        bytes[8..].copy_from_slice(&little_endian_bytes[1..8]);

        Self::try_from(bytes)
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountId {
    fn write_into<W: miden_crypto::utils::ByteWriter>(&self, target: &mut W) {
        let bytes: [u8; 15] = (*self).into();
        bytes.write_into(target);
    }

    fn get_size_hint(&self) -> usize {
        Self::SERIALIZED_SIZE
    }
}

impl Deserializable for AccountId {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        <[u8; 15]>::read_from(source)?
            .try_into()
            .map_err(|err: AccountError| DeserializationError::InvalidValue(err.to_string()))
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Returns an [AccountId] instantiated with the provided field elements.
///
/// # Errors
///
/// Returns an error if any of the ID constraints are not met. See the See the [type
/// documentation](AccountId) for details.
fn account_id_from_felts(elements: [Felt; 2]) -> Result<AccountId, AccountError> {
    validate_first_felt(elements[0])?;
    validate_second_felt(elements[1])?;

    Ok(AccountId {
        first_felt: elements[0],
        second_felt: elements[1],
    })
}

/// Checks that the first felt:
/// - has known values for metadata (storage mode, type and version).
pub(super) fn validate_first_felt(
    first_felt: Felt,
) -> Result<(AccountType, AccountStorageMode, AccountIdVersion), AccountError> {
    let first_felt = first_felt.as_int();

    // Validate storage bits.
    let storage_mode = extract_storage_mode(first_felt)?;

    // Validate version bits.
    let version = extract_version(first_felt)?;

    let account_type = extract_type(first_felt);

    Ok((account_type, storage_mode, version))
}

/// Checks that the second felt:
/// - has an anchor_epoch that is not [`u16::MAX`].
/// - has its lower 8 bits set to zero.
fn validate_second_felt(second_felt: Felt) -> Result<(), AccountError> {
    let second_felt = second_felt.as_int();

    if extract_anchor_epoch(second_felt) == u16::MAX {
        return Err(AccountError::AssumptionViolated(
            "TODO: Make proper error: second felt epoch must be less than 2^16".into(),
        ));
    }

    // Validate lower 8 bits of second felt are zero.
    if second_felt & 0xff != 0 {
        return Err(AccountError::AssumptionViolated(
            "TODO: Make proper error: second felt lower 8 bits must be zero".into(),
        ));
    }

    Ok(())
}

pub(crate) fn extract_storage_mode(first_felt: u64) -> Result<AccountStorageMode, AccountError> {
    let bits = (first_felt & AccountId::STORAGE_MODE_MASK) >> AccountId::STORAGE_MODE_SHIFT;
    match bits {
        PUBLIC => Ok(AccountStorageMode::Public),
        PRIVATE => Ok(AccountStorageMode::Private),
        _ => Err(AccountError::InvalidAccountStorageMode(format!("0b{bits:b}"))),
    }
}

pub(crate) fn extract_version(first_felt: u64) -> Result<AccountIdVersion, AccountError> {
    let bits = first_felt & AccountId::VERSION_MASK;
    let version = bits.try_into().expect("TODO");
    match version {
        AccountIdVersion::VERSION_0_NUMBER => Ok(AccountIdVersion::VERSION_0),
        other => Err(AccountError::AssumptionViolated(format!(
            "TODO: Error. Unexpected version {other}"
        ))),
    }
}

pub(crate) const fn extract_type(first_felt: u64) -> AccountType {
    let bits = (first_felt & AccountId::TYPE_MASK) >> AccountId::TYPE_SHIFT;
    match bits {
        REGULAR_ACCOUNT_UPDATABLE_CODE => AccountType::RegularAccountUpdatableCode,
        REGULAR_ACCOUNT_IMMUTABLE_CODE => AccountType::RegularAccountImmutableCode,
        FUNGIBLE_FAUCET => AccountType::FungibleFaucet,
        NON_FUNGIBLE_FAUCET => AccountType::NonFungibleFaucet,
        _ => {
            // SAFETY: type mask contains only 2 bits and we've covered all 4 possible options.
            unreachable!()
        },
    }
}

fn extract_anchor_epoch(second_felt: u64) -> u16 {
    ((second_felt & AccountId::ANCHOR_EPOCH_MASK) >> AccountId::ANCHOR_EPOCH_SHIFT) as u16
}

/// Shapes the second felt so it meets the requirements of the account ID, by overwriting the
/// upper 16 bits with the epoch and setting the lower 8 bits to zero.
fn shape_second_felt(second_felt: Felt, anchor_epoch: u16) -> Felt {
    if anchor_epoch == u16::MAX {
        unimplemented!("TODO: Return error");
    }

    let mut second_felt = second_felt.as_int();

    // Clear upper 16 epoch bits and the lower 8 bits.
    second_felt &= 0x0000_ffff_ffff_ff00;

    // Set the upper 16 anchor epoch bits.
    second_felt |= (anchor_epoch as u64) << AccountId::ANCHOR_EPOCH_SHIFT;

    // SAFETY: We disallow u16::MAX which would be all 1 bits, so at least one of the most
    // significant bits will always be zero.
    Felt::try_from(second_felt).expect("epoch is never all ones so felt should be valid")
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

/// Returns the digest of two hashing permutations over the seed, code commitment, storage
/// commitment and padding.
pub(super) fn compute_digest(
    seed: Word,
    code_commitment: Digest,
    storage_commitment: Digest,
    anchor_block_hash: Digest,
) -> Digest {
    let mut elements = Vec::with_capacity(16);
    elements.extend(seed);
    elements.extend(*code_commitment);
    elements.extend(*storage_commitment);
    elements.extend(*anchor_block_hash);
    Hasher::hash_elements(&elements)
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
    fn test_account_id_from_seed_with_epoch() {
        let code_commitment: Digest = Digest::default();
        let storage_commitment: Digest = Digest::default();
        let anchor_block_hash: Digest = Digest::default();

        let seed = AccountId::compute_account_seed(
            [10; 32],
            AccountType::FungibleFaucet,
            AccountStorageMode::Public,
            AccountIdVersion::VERSION_0,
            code_commitment,
            storage_commitment,
            anchor_block_hash,
        )
        .unwrap();

        for anchor_epoch in [0, u16::MAX - 1, 5000] {
            let anchor = AccountIdAnchor::new_unchecked(anchor_epoch, anchor_block_hash);
            let id = AccountId::new(seed, anchor, code_commitment, storage_commitment).unwrap();
            assert_eq!(id.anchor_epoch(), anchor_epoch, "failed for account id: {id}");
        }
    }

    #[test]
    fn account_id_from_felts_with_high_pop_count() {
        let valid_second_felt = Felt::try_from(0xfffe_ffff_ffff_ff00u64).unwrap();
        let valid_first_felt = Felt::try_from(0x7fff_ffff_ffff_ff00u64).unwrap();

        let id1 = AccountId::new_unchecked([valid_first_felt, valid_second_felt]);
        assert_eq!(id1.account_type(), AccountType::RegularAccountImmutableCode);
        assert_eq!(id1.storage_mode(), AccountStorageMode::Public);
        assert_eq!(id1.version(), AccountIdVersion::VERSION_0);
        assert_eq!(id1.anchor_epoch(), u16::MAX - 1);
    }

    #[test]
    fn account_id_construction() {
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
                    let id = AccountId::new_dummy(input, account_type, storage_mode);
                    assert_eq!(id.account_type(), account_type);
                    assert_eq!(id.storage_mode(), storage_mode);
                    assert_eq!(id.version(), AccountIdVersion::VERSION_0);
                    assert_eq!(id.anchor_epoch(), 0);

                    // Do a serialization roundtrip to ensure validity.
                    let serialized_id = id.to_bytes();
                    AccountId::read_from_bytes(&serialized_id).unwrap();
                    assert_eq!(serialized_id.len(), AccountId::SERIALIZED_SIZE);
                }
            }
        }
    }

    #[test]
    fn account_id_prefix_serialization_compatibility() {
        // Ensure that an AccountIdPrefix can be read from the serialized bytes of an AccountId.
        let account_id = AccountId::try_from(ACCOUNT_ID_OFF_CHAIN_SENDER).unwrap();
        let id_bytes = account_id.to_bytes();
        let deserialized_prefix = AccountIdPrefix::read_from_bytes(&id_bytes).unwrap();
        assert_eq!(account_id.prefix(), deserialized_prefix);

        // Ensure AccountId and AccountIdPrefix's hex representation are compatible.
        assert!(account_id.to_hex().starts_with(&account_id.prefix().to_string()));
    }

    // CONVERSION TESTS
    // ================================================================================================

    #[test]
    fn test_account_id_conversion_roundtrip() {
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
            let id = AccountId::try_from(account_id).expect("account ID should be valid");
            assert_eq!(id, AccountId::from_hex(&id.to_hex()).unwrap(), "failed in {idx}");
            assert_eq!(id, AccountId::try_from(<[u8; 15]>::from(id)).unwrap(), "failed in {idx}");
            assert_eq!(id, AccountId::try_from(u128::from(id)).unwrap(), "failed in {idx}");
            assert_eq!(account_id, u128::from(id), "failed in {idx}");
        }
    }

    #[test]
    fn test_account_id_tag_identifiers() {
        let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN)
            .expect("valid account ID");
        assert!(account_id.is_regular_account());
        assert_eq!(account_id.account_type(), AccountType::RegularAccountImmutableCode);
        assert!(account_id.is_public());

        let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN)
            .expect("valid account ID");
        assert!(account_id.is_regular_account());
        assert_eq!(account_id.account_type(), AccountType::RegularAccountUpdatableCode);
        assert!(!account_id.is_public());

        let account_id =
            AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).expect("valid account ID");
        assert!(account_id.is_faucet());
        assert_eq!(account_id.account_type(), AccountType::FungibleFaucet);
        assert!(account_id.is_public());

        let account_id = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN)
            .expect("valid account ID");
        assert!(account_id.is_faucet());
        assert_eq!(account_id.account_type(), AccountType::NonFungibleFaucet);
        assert!(!account_id.is_public());
    }

    /// The following test ensure there is a bit available to identify an account as a faucet or
    /// normal.
    #[test]
    fn test_account_id_faucet_bit() {
        const ACCOUNT_IS_FAUCET_MASK: u64 = 0b10;

        // faucets have a bit set
        assert_ne!((FUNGIBLE_FAUCET) & ACCOUNT_IS_FAUCET_MASK, 0);
        assert_ne!((NON_FUNGIBLE_FAUCET) & ACCOUNT_IS_FAUCET_MASK, 0);

        // normal accounts do not have the faucet bit set
        assert_eq!((REGULAR_ACCOUNT_IMMUTABLE_CODE) & ACCOUNT_IS_FAUCET_MASK, 0);
        assert_eq!((REGULAR_ACCOUNT_UPDATABLE_CODE) & ACCOUNT_IS_FAUCET_MASK, 0);
    }
}
