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
use crate::{accounts::AccountIdPrefix, AccountError, ACCOUNT_TREE_DEPTH};

// CONSTANTS
// ================================================================================================

const ACCOUNT_VERSION_MASK_SHIFT: u64 = 4;
const ACCOUNT_VERSION_MASK: u64 = 0b1111 << ACCOUNT_VERSION_MASK_SHIFT;

const ACCOUNT_BLOCK_EPOCH_MASK_SHIFT: u64 = 48;
const ACCOUNT_BLOCK_EPOCH_MASK: u64 = 0xffff << ACCOUNT_BLOCK_EPOCH_MASK_SHIFT;

// The higher two bits of the least significant nibble determines the account storage mode
const ACCOUNT_STORAGE_MASK_SHIFT: u64 = 2;
const ACCOUNT_STORAGE_MASK: u64 = 0b11 << ACCOUNT_STORAGE_MASK_SHIFT;

// The lower two bits of the least significant nibble determine the account type.
pub(super) const ACCOUNT_TYPE_MASK: u64 = 0b11;
pub const ACCOUNT_ISFAUCET_MASK: u64 = 0b10;

// ACCOUNT TYPE
// ================================================================================================

pub const FUNGIBLE_FAUCET: u64 = 0b10;
pub const NON_FUNGIBLE_FAUCET: u64 = 0b11;
pub const REGULAR_ACCOUNT_IMMUTABLE_CODE: u64 = 0b00;
pub const REGULAR_ACCOUNT_UPDATABLE_CODE: u64 = 0b01;

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

/// Extracts the [AccountType2] encoded in an u64.
///
/// The account id is encoded in the bits `[62,60]` of the u64, see [ACCOUNT_TYPE_MASK].
///
/// # Note
///
/// This function does not validate the u64, it is assumed the value is valid [Felt].
pub const fn account_type_from_u64(value: u64) -> AccountType {
    debug_assert!(
        ACCOUNT_TYPE_MASK.count_ones() == 2,
        "This method assumes there are only 2bits in the mask"
    );

    let bits = value & ACCOUNT_TYPE_MASK;
    match bits {
        REGULAR_ACCOUNT_UPDATABLE_CODE => AccountType::RegularAccountUpdatableCode,
        REGULAR_ACCOUNT_IMMUTABLE_CODE => AccountType::RegularAccountImmutableCode,
        FUNGIBLE_FAUCET => AccountType::FungibleFaucet,
        NON_FUNGIBLE_FAUCET => AccountType::NonFungibleFaucet,
        _ => {
            // account_type mask contains 2 bits and we exhaustively match all 4 possible options
            unreachable!()
        },
    }
}

// TODO: Reconsider whether we need this and if yes, whether it needs to be publicly exposed
// functionality.
/// Returns the [AccountType2] given an integer representation of `account_id`.
impl From<u128> for AccountType {
    fn from(value: u128) -> Self {
        let val = (value >> 64) as u64;
        account_type_from_u64(val)
    }
}

// ACCOUNT STORAGE TYPES
// ================================================================================================

pub const PUBLIC: u64 = 0b00;
pub const PRIVATE: u64 = 0b10;

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

// ACCOUNT ID
// ================================================================================================

/// # Layout
/// ```text
/// 1st felt: [zero bit | random (55 bits) | version (4 bits) | storage mode (2 bits) | type (2 bits)]
/// 2nd felt: [block_epoch (16 bits) | random (40 bits) | 8 zero bits]
/// ```
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AccountId {
    first_felt: Felt,
    second_felt: Felt,
}

impl AccountId {
    /// Specifies a minimum number of ones for a valid account ID.
    pub const MIN_ACCOUNT_ONES: u32 = 5;

    /// The serialized size of an [`AccountId`] in bytes.
    pub const SERIALIZED_SIZE: usize = 15;

    pub fn new(
        seed: Word,
        block_epoch: u16,
        code_commitment: Digest,
        storage_commitment: Digest,
        block_hash: Digest,
    ) -> Result<Self, AccountError> {
        let seed_digest = compute_digest(seed, code_commitment, storage_commitment, block_hash);

        let mut felts: [Felt; 2] = seed_digest.as_elements()[0..2]
            .try_into()
            .expect("we should have sliced off 2 elements");

        felts[1] = shape_second_felt(felts[1], block_epoch);

        account_id_from_felts(felts)
    }

    pub fn new_unchecked(elements: [Felt; 2]) -> Self {
        Self {
            first_felt: elements[0],
            second_felt: elements[1],
        }
    }

    #[cfg(any(feature = "testing", test))]
    pub fn new_with_type_and_mode(
        mut bytes: [u8; 15],
        account_type: AccountType,
        storage_mode: AccountStorageMode,
    ) -> AccountId {
        let version = AccountVersion::VERSION_0_NUMBER;
        let low_nibble = (version << ACCOUNT_VERSION_MASK_SHIFT)
            | (storage_mode as u8) << ACCOUNT_STORAGE_MASK_SHIFT
            | (account_type as u8);

        // Set least significant byte.
        bytes[7] = low_nibble;

        // Clear most significant bit.
        bytes[0] &= 0b0111_1111;
        // Set five one bits to satisfy MIN_ACCOUNT_ONES.
        bytes[0] |= 0b0111_1100;

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

    pub fn get_account_seed(
        init_seed: [u8; 32],
        account_type: AccountType,
        storage_mode: AccountStorageMode,
        version: AccountVersion,
        code_commitment: Digest,
        storage_commitment: Digest,
        block_hash: Digest,
    ) -> Result<Word, AccountError> {
        crate::accounts::seed::get_account_seed(
            init_seed,
            account_type,
            storage_mode,
            version,
            code_commitment,
            storage_commitment,
            block_hash,
        )
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    pub const fn account_type(&self) -> AccountType {
        extract_type(self.first_felt().as_int())
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
        extract_storage_mode(self.first_felt().as_int())
            .expect("account id should have been constructed with a valid storage mode")
    }

    /// Returns true if an account with this ID is a public account.
    pub fn is_public(&self) -> bool {
        self.storage_mode() == AccountStorageMode::Public
    }

    pub fn version(&self) -> AccountVersion {
        extract_version(self.first_felt().as_int())
            .expect("account id should have been constructed with a valid version")
    }

    pub fn block_epoch(&self) -> u16 {
        extract_block_epoch(self.second_felt().as_int())
    }

    /// Creates an Account Id from a hex string. Assumes the string starts with "0x" and
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

    pub fn prefix(&self) -> AccountIdPrefix {
        // SAFETY: We only construct accounts with valid first felts, so we don't have to validate
        // it again.
        AccountIdPrefix::new_unchecked(self.first_felt)
    }

    pub const fn first_felt(&self) -> Felt {
        self.first_felt
    }

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

    /// Returns an [AccountId] instantiated with the provided field element.
    ///
    /// # Errors
    /// Returns an error if:
    /// - If there are fewer than [AccountId2::MIN_ACCOUNT_ONES] in the provided value.
    /// - If the provided value contains invalid account ID metadata (i.e., the first 4 bits).
    fn try_from(elements: [Felt; 2]) -> Result<Self, Self::Error> {
        account_id_from_felts(elements)
    }
}

impl TryFrom<[u8; 15]> for AccountId {
    type Error = AccountError;

    /// Converts a byte array in little-endian order to an [`AccountId`].
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

/// Returns an [AccountId] instantiated with the provided field elements.
///
/// TODO
fn account_id_from_felts(elements: [Felt; 2]) -> Result<AccountId, AccountError> {
    validate_first_felt(elements[0])?;
    validate_second_felt(elements[1])?;

    Ok(AccountId {
        first_felt: elements[0],
        second_felt: elements[1],
    })
}

pub(super) fn validate_first_felt(
    first_felt: Felt,
) -> Result<(AccountType, AccountStorageMode, AccountVersion), AccountError> {
    let first_felt = first_felt.as_int();

    // Validate min account ones.
    // TODO: Describe why we only count ones on first felt.
    let ones_count = first_felt.count_ones();
    if ones_count < AccountId::MIN_ACCOUNT_ONES {
        return Err(AccountError::AccountIdTooFewOnes(ones_count));
    }

    // Validate high bit of first felt is zero.
    if first_felt >> 63 != 0 {
        return Err(AccountError::AssumptionViolated(format!(
            "TODO: Make proper error: first felt {first_felt:016x} high bit must be zero",
        )));
    }

    // Validate storage bits.
    let storage_mode = extract_storage_mode(first_felt)?;

    // Validate version bits.
    let version = extract_version(first_felt)?;

    let account_type = extract_type(first_felt);

    Ok((account_type, storage_mode, version))
}

fn validate_second_felt(second_felt: Felt) -> Result<(), AccountError> {
    let second_felt = second_felt.as_int();

    // Validate lower 8 bits of second felt are zero.
    if second_felt & 0xff != 0 {
        return Err(AccountError::AssumptionViolated(
            "TODO: Make proper error: second felt lower 8 bits must be zero".into(),
        ));
    }

    Ok(())
}

pub(super) fn extract_storage_mode(first_felt: u64) -> Result<AccountStorageMode, AccountError> {
    let bits = (first_felt & ACCOUNT_STORAGE_MASK) >> ACCOUNT_STORAGE_MASK_SHIFT;
    match bits {
        PUBLIC => Ok(AccountStorageMode::Public),
        PRIVATE => Ok(AccountStorageMode::Private),
        _ => Err(AccountError::InvalidAccountStorageMode(format!("0b{bits:b}"))),
    }
}

pub(super) fn extract_version(first_felt: u64) -> Result<AccountVersion, AccountError> {
    let bits = (first_felt & ACCOUNT_VERSION_MASK) >> ACCOUNT_VERSION_MASK_SHIFT;
    let version = bits.try_into().expect("TODO");
    match version {
        AccountVersion::VERSION_0_NUMBER => Ok(AccountVersion::VERSION_0),
        other => Err(AccountError::AssumptionViolated(format!(
            "TODO: Error. Unexpected version {other}"
        ))),
    }
}

pub(crate) const fn extract_type(first_felt: u64) -> AccountType {
    let bits = first_felt & ACCOUNT_TYPE_MASK;
    match bits {
        REGULAR_ACCOUNT_UPDATABLE_CODE => AccountType::RegularAccountUpdatableCode,
        REGULAR_ACCOUNT_IMMUTABLE_CODE => AccountType::RegularAccountImmutableCode,
        FUNGIBLE_FAUCET => AccountType::FungibleFaucet,
        NON_FUNGIBLE_FAUCET => AccountType::NonFungibleFaucet,
        _ => {
            // account_type mask contains only 2bits, there are 4 options total.
            unreachable!()
        },
    }
}

fn extract_block_epoch(second_felt: u64) -> u16 {
    ((second_felt & ACCOUNT_BLOCK_EPOCH_MASK) >> ACCOUNT_BLOCK_EPOCH_MASK_SHIFT) as u16
}

/// Shapes the second felt so it meets the requirements of the account ID, by overwriting the
/// upper 16 bits with the epoch and setting the lower 8 bits to zero.
fn shape_second_felt(second_felt: Felt, block_epoch: u16) -> Felt {
    if block_epoch == u16::MAX {
        unimplemented!("TODO: Return error");
    }

    // Set epoch and set lower 8 bits to zero.
    let mut second_felt = second_felt.as_int();
    let block_epoch_u64 = (block_epoch as u64) << ACCOUNT_BLOCK_EPOCH_MASK_SHIFT;
    let block_epoch_mask = 0x0000_ffff_ffff_ff00 | block_epoch_u64;

    second_felt &= block_epoch_mask;

    // SAFETY: We disallow u16::MAX which would be all 1 bits, so at least one of the most
    // significant bits will always be zero.
    Felt::try_from(second_felt).expect("epoch is never all ones so felt should be valid")
}

impl PartialOrd for AccountId {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AccountId {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        let self_int: u128 = (*self).into();
        let other_int: u128 = (*other).into();
        self_int.cmp(&other_int)
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct AccountVersion(u8);

impl AccountVersion {
    const VERSION_0_NUMBER: u8 = 0;
    pub const VERSION_0: AccountVersion = AccountVersion(Self::VERSION_0_NUMBER);

    pub const fn version_num(&self) -> u8 {
        self.0
    }
}

/// Returns the digest of two hashing permutations over the seed, code commitment, storage
/// commitment and padding.
pub(super) fn compute_digest(
    seed: Word,
    code_commitment: Digest,
    storage_commitment: Digest,
    block_hash: Digest,
) -> Digest {
    let mut elements = Vec::with_capacity(16);
    elements.extend(seed);
    elements.extend(*code_commitment);
    elements.extend(*storage_commitment);
    elements.extend(*block_hash);
    Hasher::hash_elements(&elements)
}

// TESTING
// ================================================================================================

#[cfg(any(feature = "testing", test))]
pub mod testing {
    use super::{AccountStorageMode, AccountType, ACCOUNT_STORAGE_MASK_SHIFT};

    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    // REGULAR ACCOUNTS - OFF-CHAIN
    pub const ACCOUNT_ID_SENDER: u128 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageMode::Private,
        0xaabb_ccdd,
    );
    pub const ACCOUNT_ID_OFF_CHAIN_SENDER: u128 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageMode::Private,
        0xbbcc_ddee,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN: u128 = account_id(
        AccountType::RegularAccountUpdatableCode,
        AccountStorageMode::Private,
        0xccdd_eeff,
    );
    // REGULAR ACCOUNTS - ON-CHAIN
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN: u128 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageMode::Public,
        0xaabb_ccdd,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2: u128 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageMode::Public,
        0xbbcc_ddee,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: u128 = account_id(
        AccountType::RegularAccountUpdatableCode,
        AccountStorageMode::Public,
        0xccdd_eeff,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2: u128 = account_id(
        AccountType::RegularAccountUpdatableCode,
        AccountStorageMode::Public,
        0xeeff_ccdd,
    );

    // FUNGIBLE TOKENS - OFF-CHAIN
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN: u128 =
        account_id(AccountType::FungibleFaucet, AccountStorageMode::Private, 0xaabb_ccdd);
    // FUNGIBLE TOKENS - ON-CHAIN
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u128 =
        account_id(AccountType::FungibleFaucet, AccountStorageMode::Public, 0xaabb_ccdd);
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1: u128 =
        account_id(AccountType::FungibleFaucet, AccountStorageMode::Public, 0xbbcc_ddee);
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2: u128 =
        account_id(AccountType::FungibleFaucet, AccountStorageMode::Public, 0xccdd_eeff);
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3: u128 =
        account_id(AccountType::FungibleFaucet, AccountStorageMode::Public, 0xeeff_cc99);

    // NON-FUNGIBLE TOKENS - OFF-CHAIN
    pub const ACCOUNT_ID_INSUFFICIENT_ONES: u128 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageMode::Private, 0b0000_0000); // invalid
    pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN: u128 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageMode::Private, 0xaabb_ccdd);
    // NON-FUNGIBLE TOKENS - ON-CHAIN
    pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN: u128 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageMode::Public, 0xbbcc_ddee);
    pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1: u128 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageMode::Public, 0xccdd_eeff);

    // TEST ACCOUNT IDs WITH CERTAIN PROPERTIES
    /// The Account Id with the maximum possible one bits.
    pub const ACCOUNT_ID_MAX_ONES: u128 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageMode::Private, 0)
            | 0x7fff_ffff_ffff_ff00_7fff_ffff_ffff_ff00;
    /// The Account Id with the maximum possible zeroe bits.
    pub const ACCOUNT_ID_MAX_ZEROES: u128 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageMode::Private, 0x001f_0000);

    // UTILITIES
    // --------------------------------------------------------------------------------------------

    /// Produces a valid account ID with the given account type and storage mode.
    ///
    /// - Version is set to 0.
    /// - Epoch is set to 0.
    /// - The 2nd most significant bit is set to 1, so it is easier to test the note_tag, for
    ///   example.
    ///
    /// Finally, distributes the given `random` value over the ID to produce reasonably realistic
    /// values. This is easiest explained with an example. Suppose `random` is `0xaabb_ccdd`,
    /// then the layout of the generated ID will be:
    ///
    /// ```text
    /// 1st felt: [0b0100_0000 | 0xaa | 4 zero bytes | 0xbb | metadata byte]
    /// 2nd felt: [2 zero bytes (epoch) | 0xcc | 3 zero bytes | 0xdd | zero byte]
    /// ```
    pub const fn account_id(
        account_type: AccountType,
        storage_mode: AccountStorageMode,
        random: u32,
    ) -> u128 {
        let mut first_felt: u64 = 0;

        first_felt |= account_type as u64;
        first_felt |= (storage_mode as u64) << ACCOUNT_STORAGE_MASK_SHIFT;

        // Produce more realistic IDs by distributing the random value.
        let random_1st_felt_upper = random & 0xff00_0000;
        let random_1st_felt_lower = random & 0x00ff_0000;
        let random_2nd_felt_upper = random & 0x0000_ff00;
        let random_2nd_felt_lower = random & 0x0000_00ff;

        // Shift the random part of the ID to start at the most significant end.
        first_felt |= (random_1st_felt_upper as u64) << 24;
        first_felt |= (random_1st_felt_lower as u64) >> 8;

        let mut id = (first_felt as u128) << 64;

        id |= (random_2nd_felt_upper as u128) << 32;
        id |= (random_2nd_felt_lower as u128) << 8;

        id
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {

    use vm_core::StarkField;

    use super::*;
    use crate::accounts::testing::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
        ACCOUNT_ID_OFF_CHAIN_SENDER, ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
    };

    #[test]
    fn test_account_id_validation() {
        let felt_max: Felt = Felt::new(Felt::MODULUS);
        AccountId::try_from([felt_max, felt_max]).unwrap_err();
    }

    #[test]
    fn test_account_id_from_seed_with_epoch() {
        let code_commitment: Digest = Digest::default();
        let storage_commitment: Digest = Digest::default();
        let block_hash: Digest = Digest::default();

        let seed = AccountId::get_account_seed(
            [10; 32],
            AccountType::FungibleFaucet,
            AccountStorageMode::Public,
            AccountVersion::VERSION_0,
            code_commitment,
            storage_commitment,
            block_hash,
        )
        .unwrap();

        for block_epoch in [0, u16::MAX - 1, 5000] {
            let id =
                AccountId::new(seed, block_epoch, code_commitment, storage_commitment, block_hash)
                    .unwrap();
            assert_eq!(id.block_epoch(), block_epoch);
        }
    }

    #[test]
    fn test_account_id() {
        let valid_second_felt = Felt::try_from(0xfffe_ffff_ffff_ff00u64).unwrap();
        let valid_first_felt = Felt::try_from(0x7fff_ffff_ffff_ff00u64).unwrap();

        let id1 = AccountId::new_unchecked([valid_first_felt, valid_second_felt]);
        assert_eq!(id1.account_type(), AccountType::RegularAccountImmutableCode);
        assert_eq!(id1.storage_mode(), AccountStorageMode::Public);
        assert_eq!(id1.version(), AccountVersion::VERSION_0);
        assert_eq!(id1.block_epoch(), u16::MAX - 1);
    }

    #[test]
    fn account_id_construction() {
        // Use the highest possible input to check if the constructed id is a valid Felt in that
        // scenario.
        // Use the lowest possible input to check whether the constructor satisfies
        // MIN_ACCOUNT_ONES.
        for input in [[0xff; 15], [0; 15]] {
            for account_type in [
                AccountType::FungibleFaucet,
                AccountType::NonFungibleFaucet,
                AccountType::RegularAccountImmutableCode,
                AccountType::RegularAccountUpdatableCode,
            ] {
                for storage_mode in [AccountStorageMode::Private, AccountStorageMode::Public] {
                    let id = AccountId::new_with_type_and_mode(input, account_type, storage_mode);
                    assert_eq!(id.account_type(), account_type);
                    assert_eq!(id.storage_mode(), storage_mode);
                    assert_eq!(id.block_epoch(), 0);

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
    fn test_account_id_account_type() {
        let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN)
            .expect("valid account ID");

        let account_type: AccountType = ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.into();
        assert_eq!(account_type, account_id.account_type());

        let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN)
            .expect("valid account ID");
        let account_type: AccountType = ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN.into();
        assert_eq!(account_type, account_id.account_type());

        let account_id =
            AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).expect("valid account ID");
        let account_type: AccountType = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.into();
        assert_eq!(account_type, account_id.account_type());

        let account_id = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN)
            .expect("valid account ID");
        let account_type: AccountType = ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN.into();
        assert_eq!(account_type, account_id.account_type());
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
