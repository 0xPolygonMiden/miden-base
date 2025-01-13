use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use miden_crypto::{merkle::LeafIndex, utils::hex_to_bytes};
use vm_core::{
    utils::{ByteReader, Deserializable, Serializable},
    Felt, Word,
};
use vm_processor::{DeserializationError, Digest};

use crate::{
    accounts::{
        account_id::{
            account_type::{
                FUNGIBLE_FAUCET, NON_FUNGIBLE_FAUCET, REGULAR_ACCOUNT_IMMUTABLE_CODE,
                REGULAR_ACCOUNT_UPDATABLE_CODE,
            },
            storage_mode::{PRIVATE, PUBLIC},
        },
        AccountIdAnchor, AccountIdPrefix, AccountIdVersion, AccountStorageMode, AccountType,
    },
    errors::AccountIdError,
    AccountError, Hasher, ACCOUNT_TREE_DEPTH,
};

// ACCOUNT ID VERSION 0
// ================================================================================================

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AccountIdV0 {
    prefix: Felt,
    suffix: Felt,
}

impl AccountIdV0 {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// The serialized size of an [`AccountIdV0`] in bytes.
    pub const SERIALIZED_SIZE: usize = 15;

    /// The lower two bits of the second least significant nibble determine the account type.
    pub(crate) const TYPE_SHIFT: u64 = 4;
    pub(crate) const TYPE_MASK: u8 = 0b11 << Self::TYPE_SHIFT;

    /// The least significant nibble determines the account version.
    const VERSION_MASK: u64 = 0b1111;

    const ANCHOR_EPOCH_SHIFT: u64 = 48;
    const ANCHOR_EPOCH_MASK: u64 = 0xffff << Self::ANCHOR_EPOCH_SHIFT;

    /// The higher two bits of the second least significant nibble determine the account storage
    /// mode.
    pub(crate) const STORAGE_MODE_SHIFT: u64 = 6;
    pub(crate) const STORAGE_MODE_MASK: u8 = 0b11 << Self::STORAGE_MODE_SHIFT;

    pub(crate) const IS_FAUCET_MASK: u64 = 0b10 << Self::TYPE_SHIFT;

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates an [`AccountIdV0`] by hashing the given `seed`, `code_commitment`,
    /// `storage_commitment` and [`AccountIdAnchor::block_hash`] from the `anchor` and using the
    /// resulting first and second element of the hash as the prefix and suffix felts of the ID.
    /// The [`AccountIdAnchor::epoch`] from the `anchor` overwrites part of the suffix.
    ///
    /// Note that the `anchor` must correspond to a valid block in the chain for the ID to be deemed
    /// valid during creation.
    ///
    /// See the documentation of the [`AccountIdV0`] for more details on the generation.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints are not met. See the [type
    /// documentation](AccountIdV0) for details.
    pub fn new(
        seed: Word,
        anchor: AccountIdAnchor,
        code_commitment: Digest,
        storage_commitment: Digest,
    ) -> Result<Self, AccountIdError> {
        let seed_digest =
            compute_digest(seed, code_commitment, storage_commitment, anchor.block_hash());

        let mut felts: [Felt; 2] = seed_digest.as_elements()[0..2]
            .try_into()
            .expect("we should have sliced off 2 elements");

        felts[1] = shape_suffix(felts[1], anchor.epoch())?;

        // This will validate that the anchor_epoch we have just written is not u16::MAX.
        account_id_from_felts(felts)
    }

    /// Creates an [`AccountIdV0`] from the given felts where the felt at index 0 is the prefix
    /// and the felt at index 2 is the suffix.
    ///
    /// # Warning
    ///
    /// Validity of the ID must be ensured by the caller. An invalid ID may lead to panics.
    ///
    /// # Panics
    ///
    /// If debug_assertions are enabled (e.g. in debug mode), this function panics if any of the ID
    /// constraints are not met. See the [constraints documentation](super::AccountId#constraints)
    /// for details.
    pub fn new_unchecked(elements: [Felt; 2]) -> Self {
        let prefix = elements[0];
        let suffix = elements[1];

        // Panic on invalid felts in debug mode.
        if cfg!(debug_assertions) {
            validate_prefix(prefix).expect("AccountId::new_unchecked called with invalid prefix");
            validate_suffix(suffix).expect("AccountId::new_unchecked called with invalid suffix");
        }

        Self { prefix, suffix }
    }

    /// Constructs an [`AccountIdV0`] for testing purposes with the given account type and storage
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
        mut bytes: [u8; 15],
        account_type: AccountType,
        storage_mode: AccountStorageMode,
    ) -> AccountIdV0 {
        let version = AccountIdVersion::Version0 as u8;
        let low_nibble = (storage_mode as u8) << Self::STORAGE_MODE_SHIFT
            | (account_type as u8) << Self::TYPE_SHIFT
            | version;

        // Set least significant byte.
        bytes[7] = low_nibble;

        // Clear the 32nd most significant bit.
        bytes[3] &= 0b1111_1110;

        let prefix_bytes =
            bytes[0..8].try_into().expect("we should have sliced off exactly 8 bytes");
        let prefix = Felt::try_from(u64::from_be_bytes(prefix_bytes))
            .expect("should be a valid felt due to the most significant bit being zero");

        let mut suffix_bytes = [0; 8];
        // Overwrite first 7 bytes, leaving the 8th byte 0 (which will be cleared by
        // shape_suffix anyway).
        suffix_bytes[..7].copy_from_slice(&bytes[8..]);
        // If the value is too large modular reduction is performed, which is fine here.
        let mut suffix = Felt::new(u64::from_be_bytes(suffix_bytes));

        suffix = shape_suffix(suffix, 0).expect("anchor epoch is not u16::MAX");

        let account_id = account_id_from_felts([prefix, suffix])
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
        crate::accounts::account_id::seed::compute_account_seed(
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
        extract_type(self.prefix.as_int())
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
        extract_storage_mode(self.prefix().as_u64())
            .expect("account ID should have been constructed with a valid storage mode")
    }

    /// Returns true if an account with this ID is a public account.
    pub fn is_public(&self) -> bool {
        self.storage_mode() == AccountStorageMode::Public
    }

    /// Returns the version of this account ID.
    pub fn version(&self) -> AccountIdVersion {
        extract_version(self.prefix().as_u64())
            .expect("account ID should have been constructed with a valid version")
    }

    /// Returns the anchor epoch of this account ID.
    ///
    /// This is the epoch to which this ID is anchored. The hash of this epoch block is used in the
    /// generation of the ID.
    pub fn anchor_epoch(&self) -> u16 {
        extract_anchor_epoch(self.suffix().as_int())
    }

    /// Creates an [`AccountIdV0`] from a hex string. Assumes the string starts with "0x" and
    /// that the hexadecimal characters are big-endian encoded.
    pub fn from_hex(hex_str: &str) -> Result<AccountIdV0, AccountIdError> {
        hex_to_bytes(hex_str)
            .map_err(AccountIdError::AccountIdHexParseError)
            .and_then(AccountIdV0::try_from)
    }

    /// Returns a big-endian, hex-encoded string of length 32, including the `0x` prefix. This means
    /// it encodes 15 bytes.
    pub fn to_hex(self) -> String {
        // We need to pad the suffix with 16 zeroes so it produces a correctly padded 8 byte
        // big-endian hex string. Only then can we cut off the last zero byte by truncating. We
        // cannot use `:014x` padding.
        let mut hex_string =
            format!("0x{:016x}{:016x}", self.prefix().as_u64(), self.suffix().as_int());
        hex_string.truncate(32);
        hex_string
    }

    /// Returns the [`AccountIdPrefix`] of this ID.
    ///
    /// The prefix of an account ID is guaranteed to be unique.
    pub fn prefix(&self) -> AccountIdPrefix {
        // SAFETY: We only construct account IDs with valid prefixes, so we don't have to validate
        // it again.
        AccountIdPrefix::new_unchecked(self.prefix)
    }

    /// Returns the suffix of this ID as a [`Felt`].
    pub const fn suffix(&self) -> Felt {
        self.suffix
    }
}

// CONVERSIONS FROM ACCOUNT ID
// ================================================================================================

impl From<AccountIdV0> for [Felt; 2] {
    fn from(id: AccountIdV0) -> Self {
        [id.prefix, id.suffix]
    }
}

impl From<AccountIdV0> for [u8; 15] {
    fn from(id: AccountIdV0) -> Self {
        let mut result = [0_u8; 15];
        result[..8].copy_from_slice(&id.prefix().as_u64().to_be_bytes());
        // The last byte of the suffix is always zero so we skip it here.
        result[8..].copy_from_slice(&id.suffix().as_int().to_be_bytes()[..7]);
        result
    }
}

impl From<AccountIdV0> for u128 {
    fn from(id: AccountIdV0) -> Self {
        let mut le_bytes = [0_u8; 16];
        le_bytes[..8].copy_from_slice(&id.suffix().as_int().to_le_bytes());
        le_bytes[8..].copy_from_slice(&id.prefix().as_u64().to_le_bytes());
        u128::from_le_bytes(le_bytes)
    }
}

/// Account IDs are used as indexes in the account database, which is a tree of depth 64.
impl From<AccountIdV0> for LeafIndex<ACCOUNT_TREE_DEPTH> {
    fn from(id: AccountIdV0) -> Self {
        LeafIndex::new_max_depth(id.prefix().as_u64())
    }
}

// CONVERSIONS TO ACCOUNT ID
// ================================================================================================

impl TryFrom<[Felt; 2]> for AccountIdV0 {
    type Error = AccountIdError;

    /// Returns an [`AccountIdV0`] instantiated with the provided field elements where `elements[0]`
    /// is taken as the prefix and `elements[1]` is taken as the second element.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints are not met. See the [type
    /// documentation](AccountIdV0) for details.
    fn try_from(elements: [Felt; 2]) -> Result<Self, Self::Error> {
        account_id_from_felts(elements)
    }
}

impl TryFrom<[u8; 15]> for AccountIdV0 {
    type Error = AccountIdError;

    /// Tries to convert a byte array in big-endian order to an [`AccountIdV0`].
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints are not met. See the [type
    /// documentation](AccountIdV0) for details.
    fn try_from(mut bytes: [u8; 15]) -> Result<Self, Self::Error> {
        // Felt::try_from expects little-endian order, so reverse the individual felt slices.
        // This prefix slice has 8 bytes.
        bytes[..8].reverse();
        // The suffix slice has 7 bytes, since the 8th byte will always be zero.
        bytes[8..15].reverse();

        let prefix_slice = &bytes[..8];
        let suffix_slice = &bytes[8..15];

        // The byte order is little-endian here, so we prepend a 0 to set the least significant
        // byte.
        let mut suffix_bytes = [0; 8];
        suffix_bytes[1..8].copy_from_slice(suffix_slice);

        let prefix = Felt::try_from(prefix_slice)
            .map_err(AccountIdError::AccountIdInvalidPrefixFieldElement)?;

        let suffix = Felt::try_from(suffix_bytes.as_slice())
            .map_err(AccountIdError::AccountIdInvalidSuffixFieldElement)?;

        Self::try_from([prefix, suffix])
    }
}

impl TryFrom<u128> for AccountIdV0 {
    type Error = AccountIdError;

    /// Tries to convert a u128 into an [`AccountIdV0`].
    ///
    /// # Errors
    ///
    /// Returns an error if any of the ID constraints are not met. See the [type
    /// documentation](AccountIdV0) for details.
    fn try_from(int: u128) -> Result<Self, Self::Error> {
        let mut bytes: [u8; 15] = [0; 15];
        bytes.copy_from_slice(&int.to_be_bytes()[0..15]);

        Self::try_from(bytes)
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountIdV0 {
    fn write_into<W: miden_crypto::utils::ByteWriter>(&self, target: &mut W) {
        let bytes: [u8; 15] = (*self).into();
        bytes.write_into(target);
    }

    fn get_size_hint(&self) -> usize {
        Self::SERIALIZED_SIZE
    }
}

impl Deserializable for AccountIdV0 {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        <[u8; 15]>::read_from(source)?
            .try_into()
            .map_err(|err: AccountIdError| DeserializationError::InvalidValue(err.to_string()))
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Returns an [AccountId] instantiated with the provided field elements.
///
/// # Errors
///
/// Returns an error if any of the ID constraints are not met. See the See the [type
/// documentation](AccountIdV0) for details.
fn account_id_from_felts(elements: [Felt; 2]) -> Result<AccountIdV0, AccountIdError> {
    validate_prefix(elements[0])?;
    validate_suffix(elements[1])?;

    Ok(AccountIdV0 { prefix: elements[0], suffix: elements[1] })
}

/// Checks that the prefix:
/// - has known values for metadata (storage mode, type and version).
pub(crate) fn validate_prefix(
    prefix: Felt,
) -> Result<(AccountType, AccountStorageMode, AccountIdVersion), AccountIdError> {
    let prefix = prefix.as_int();

    // Validate storage bits.
    let storage_mode = extract_storage_mode(prefix)?;

    // Validate version bits.
    let version = extract_version(prefix)?;

    let account_type = extract_type(prefix);

    Ok((account_type, storage_mode, version))
}

/// Checks that the suffix:
/// - has an anchor_epoch that is not [`u16::MAX`].
/// - has its lower 8 bits set to zero.
const fn validate_suffix(suffix: Felt) -> Result<(), AccountIdError> {
    let suffix = suffix.as_int();

    if extract_anchor_epoch(suffix) == u16::MAX {
        return Err(AccountIdError::AnchorEpochMustNotBeU16Max);
    }

    // Validate lower 8 bits of second felt are zero.
    if suffix & 0xff != 0 {
        return Err(AccountIdError::AccountIdSuffixLeastSignificantByteMustBeZero);
    }

    Ok(())
}

pub(crate) fn extract_storage_mode(prefix: u64) -> Result<AccountStorageMode, AccountIdError> {
    let bits = (prefix & AccountIdV0::STORAGE_MODE_MASK as u64) >> AccountIdV0::STORAGE_MODE_SHIFT;
    // SAFETY: `STORAGE_MODE_MASK` is u8 so casting bits is lossless
    match bits as u8 {
        PUBLIC => Ok(AccountStorageMode::Public),
        PRIVATE => Ok(AccountStorageMode::Private),
        _ => Err(AccountIdError::UnknownAccountStorageMode(format!("0b{bits:b}").into())),
    }
}

pub(crate) fn extract_version(prefix: u64) -> Result<AccountIdVersion, AccountIdError> {
    // SAFETY: The mask guarantees that we only mask out the least significant nibble, so casting to
    // u8 is safe.
    let version = (prefix & AccountIdV0::VERSION_MASK) as u8;
    AccountIdVersion::try_from(version)
}

pub(crate) const fn extract_type(prefix: u64) -> AccountType {
    let bits = (prefix & (AccountIdV0::TYPE_MASK as u64)) >> AccountIdV0::TYPE_SHIFT;
    // SAFETY: `TYPE_MASK` is u8 so casting bits is lossless
    match bits as u8 {
        REGULAR_ACCOUNT_UPDATABLE_CODE => AccountType::RegularAccountUpdatableCode,
        REGULAR_ACCOUNT_IMMUTABLE_CODE => AccountType::RegularAccountImmutableCode,
        FUNGIBLE_FAUCET => AccountType::FungibleFaucet,
        NON_FUNGIBLE_FAUCET => AccountType::NonFungibleFaucet,
        _ => {
            // SAFETY: type mask contains only 2 bits and we've covered all 4 possible options.
            panic!("type mask contains only 2 bits and we've covered all 4 possible options")
        },
    }
}

const fn extract_anchor_epoch(suffix: u64) -> u16 {
    ((suffix & AccountIdV0::ANCHOR_EPOCH_MASK) >> AccountIdV0::ANCHOR_EPOCH_SHIFT) as u16
}

/// Shapes the suffix so it meets the requirements of the account ID, by overwriting the
/// upper 16 bits with the epoch and setting the lower 8 bits to zero.
fn shape_suffix(suffix: Felt, anchor_epoch: u16) -> Result<Felt, AccountIdError> {
    if anchor_epoch == u16::MAX {
        return Err(AccountIdError::AnchorEpochMustNotBeU16Max);
    }

    let mut suffix = suffix.as_int();

    // Clear upper 16 epoch bits and the lower 8 bits.
    suffix &= 0x0000_ffff_ffff_ff00;

    // Set the upper 16 anchor epoch bits.
    suffix |= (anchor_epoch as u64) << AccountIdV0::ANCHOR_EPOCH_SHIFT;

    // SAFETY: We disallow u16::MAX which would be all 1 bits, so at least one of the most
    // significant bits will always be zero.
    Ok(Felt::try_from(suffix).expect("epoch is never all ones so felt should be valid"))
}

// COMMON TRAIT IMPLS
// ================================================================================================

impl PartialOrd for AccountIdV0 {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AccountIdV0 {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        u128::from(*self).cmp(&u128::from(*other))
    }
}

impl fmt::Display for AccountIdV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Returns the digest of two hashing permutations over the seed, code commitment, storage
/// commitment and padding.
pub(crate) fn compute_digest(
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

        let seed = AccountIdV0::compute_account_seed(
            [10; 32],
            AccountType::FungibleFaucet,
            AccountStorageMode::Public,
            AccountIdVersion::Version0,
            code_commitment,
            storage_commitment,
            anchor_block_hash,
        )
        .unwrap();

        for anchor_epoch in [0, u16::MAX - 1, 5000] {
            let anchor = AccountIdAnchor::new_unchecked(anchor_epoch, anchor_block_hash);
            let id = AccountIdV0::new(seed, anchor, code_commitment, storage_commitment).unwrap();
            assert_eq!(id.anchor_epoch(), anchor_epoch, "failed for account ID: {id}");
        }
    }

    #[test]
    fn account_id_from_felts_with_high_pop_count() {
        let valid_suffix = Felt::try_from(0xfffe_ffff_ffff_ff00u64).unwrap();
        let valid_prefix = Felt::try_from(0x7fff_ffff_ffff_ff00u64).unwrap();

        let id1 = AccountIdV0::new_unchecked([valid_prefix, valid_suffix]);
        assert_eq!(id1.account_type(), AccountType::RegularAccountImmutableCode);
        assert_eq!(id1.storage_mode(), AccountStorageMode::Public);
        assert_eq!(id1.version(), AccountIdVersion::Version0);
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
                    let id = AccountIdV0::dummy(input, account_type, storage_mode);
                    assert_eq!(id.account_type(), account_type);
                    assert_eq!(id.storage_mode(), storage_mode);
                    assert_eq!(id.version(), AccountIdVersion::Version0);
                    assert_eq!(id.anchor_epoch(), 0);

                    // Do a serialization roundtrip to ensure validity.
                    let serialized_id = id.to_bytes();
                    AccountIdV0::read_from_bytes(&serialized_id).unwrap();
                    assert_eq!(serialized_id.len(), AccountIdV0::SERIALIZED_SIZE);
                }
            }
        }
    }

    #[test]
    fn account_id_prefix_serialization_compatibility() {
        // Ensure that an AccountIdPrefix can be read from the serialized bytes of an AccountId.
        let account_id = AccountIdV0::try_from(ACCOUNT_ID_OFF_CHAIN_SENDER).unwrap();
        let id_bytes = account_id.to_bytes();
        assert_eq!(account_id.prefix().to_bytes(), id_bytes[..8]);

        let deserialized_prefix = AccountIdPrefix::read_from_bytes(&id_bytes).unwrap();
        assert_eq!(account_id.prefix(), deserialized_prefix);

        // Ensure AccountId and AccountIdPrefix's hex representation are compatible.
        assert!(account_id.to_hex().starts_with(&account_id.prefix().to_hex()));
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
            let id = AccountIdV0::try_from(account_id).expect("account ID should be valid");
            assert_eq!(id, AccountIdV0::from_hex(&id.to_hex()).unwrap(), "failed in {idx}");
            assert_eq!(id, AccountIdV0::try_from(<[u8; 15]>::from(id)).unwrap(), "failed in {idx}");
            assert_eq!(id, AccountIdV0::try_from(u128::from(id)).unwrap(), "failed in {idx}");
            // The u128 big-endian representation without the least significant byte and the
            // [u8; 15] representations should be equivalent.
            assert_eq!(u128::from(id).to_be_bytes()[0..15], <[u8; 15]>::from(id));
            assert_eq!(account_id, u128::from(id), "failed in {idx}");
        }
    }

    #[test]
    fn test_account_id_tag_identifiers() {
        let account_id = AccountIdV0::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN)
            .expect("valid account ID");
        assert!(account_id.is_regular_account());
        assert_eq!(account_id.account_type(), AccountType::RegularAccountImmutableCode);
        assert!(account_id.is_public());

        let account_id = AccountIdV0::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN)
            .expect("valid account ID");
        assert!(account_id.is_regular_account());
        assert_eq!(account_id.account_type(), AccountType::RegularAccountUpdatableCode);
        assert!(!account_id.is_public());

        let account_id =
            AccountIdV0::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).expect("valid account ID");
        assert!(account_id.is_faucet());
        assert_eq!(account_id.account_type(), AccountType::FungibleFaucet);
        assert!(account_id.is_public());

        let account_id = AccountIdV0::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN)
            .expect("valid account ID");
        assert!(account_id.is_faucet());
        assert_eq!(account_id.account_type(), AccountType::NonFungibleFaucet);
        assert!(!account_id.is_public());
    }
}
