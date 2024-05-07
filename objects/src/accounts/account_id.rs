use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::{self, Display};

use super::{
    get_account_seed, AccountError, ByteReader, Deserializable, DeserializationError, Digest, Felt,
    Hasher, Serializable, Word, ZERO,
};
use crate::{crypto::merkle::LeafIndex, utils::hex_to_bytes, ACCOUNT_TREE_DEPTH};

// MASKS
// ================================================================================================

pub const ACCOUNT_STORAGE_MASK_SHIFT: u64 = 62;
pub const ACCOUNT_STORAGE_MASK: u64 = 0b11 << ACCOUNT_STORAGE_MASK_SHIFT;

pub const ACCOUNT_TYPE_MASK_SHIFT: u64 = 60;
pub const ACCOUNT_TYPE_MASK: u64 = 0b11 << ACCOUNT_TYPE_MASK_SHIFT;

pub const ACCOUNT_POW_MASK_SHIFT: u64 = 4;
pub const ACCOUNT_POW_MASK: u64 = ACCOUNT_POW_MAXIMUM << ACCOUNT_POW_MASK_SHIFT;

pub const ACCOUNT_RANDOM_BITS_MASK_SHIFT: u64 = 10;
pub const ACCOUNT_RANDOM_BITS_MASK: u64 =
    0b1111111111_1111111111_1111111111_1111111111_1111111111 << ACCOUNT_RANDOM_BITS_MASK_SHIFT;

pub const ACCOUNT_CONFIG_MASK: u64 = ACCOUNT_STORAGE_MASK | ACCOUNT_TYPE_MASK | ACCOUNT_POW_MASK;

pub const ACCOUNT_ZERO_MASK: u64 = 0b1111;
pub const ACCOUNT_NON_RANDOM_MASK: u64 =
    ACCOUNT_STORAGE_MASK | ACCOUNT_TYPE_MASK | ACCOUNT_POW_MASK | ACCOUNT_ZERO_MASK;

// BIT PATTERNS
// ================================================================================================

pub const ACCOUNT_ISFAUCET_BIT: u64 = 0b10 << ACCOUNT_TYPE_MASK_SHIFT;
pub const ACCOUNT_OFF_CHAIN: u64 = 0b10 << ACCOUNT_STORAGE_MASK_SHIFT;
pub const ACCOUNT_ON_CHAIN: u64 = 0b00 << ACCOUNT_STORAGE_MASK_SHIFT;
pub const FUNGIBLE_FAUCET: u64 = 0b10 << ACCOUNT_TYPE_MASK_SHIFT;
pub const NON_FUNGIBLE_FAUCET: u64 = 0b11 << ACCOUNT_TYPE_MASK_SHIFT;
pub const REGULAR_ACCOUNT_IMMUTABLE_CODE: u64 = 0b00 << ACCOUNT_TYPE_MASK_SHIFT;
pub const REGULAR_ACCOUNT_UPDATABLE_CODE: u64 = 0b01 << ACCOUNT_TYPE_MASK_SHIFT;

// CONSTANTS
// ================================================================================================

pub const ACCOUNT_POW_MAXIMUM: u64 = 0b111111;
pub const REGULAR_ACCOUNT_MINIMUM_POW: u8 = if cfg!(feature = "testing") { 5 } else { 24 };
pub const FAUCET_ACCOUNT_MINIMUM_POW: u8 = if cfg!(feature = "testing") { 7 } else { 35 };

// ACCOUNT TYPE
// ================================================================================================

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum AccountType {
    FungibleFaucet = FUNGIBLE_FAUCET,
    NonFungibleFaucet = NON_FUNGIBLE_FAUCET,
    RegularAccountImmutableCode = REGULAR_ACCOUNT_IMMUTABLE_CODE,
    RegularAccountUpdatableCode = REGULAR_ACCOUNT_UPDATABLE_CODE,
}

impl AccountType {
    pub const fn required_pow(&self) -> AccountPoW {
        match self {
            AccountType::NonFungibleFaucet | AccountType::FungibleFaucet => {
                AccountPoW(FAUCET_ACCOUNT_MINIMUM_POW)
            },
            AccountType::RegularAccountUpdatableCode | AccountType::RegularAccountImmutableCode => {
                AccountPoW(REGULAR_ACCOUNT_MINIMUM_POW)
            },
        }
    }
}

impl Display for AccountType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountType::FungibleFaucet => write!(f, "fungible faucet"),
            AccountType::NonFungibleFaucet => write!(f, "non-fungible faucet"),
            AccountType::RegularAccountImmutableCode => write!(f, "immutable regular account"),
            AccountType::RegularAccountUpdatableCode => write!(f, "updatable regular account"),
        }
    }
}

impl TryFrom<u64> for AccountType {
    type Error = AccountError;

    fn try_from(value: u64) -> Result<Self, AccountError> {
        match value {
            REGULAR_ACCOUNT_UPDATABLE_CODE => Ok(AccountType::RegularAccountUpdatableCode),
            REGULAR_ACCOUNT_IMMUTABLE_CODE => Ok(AccountType::RegularAccountImmutableCode),
            FUNGIBLE_FAUCET => Ok(AccountType::FungibleFaucet),
            NON_FUNGIBLE_FAUCET => Ok(AccountType::NonFungibleFaucet),
            v => Err(AccountError::InvalidAccountType(v)),
        }
    }
}

impl TryFrom<Felt> for AccountType {
    type Error = AccountError;

    fn try_from(value: Felt) -> Result<Self, AccountError> {
        value.as_int().try_into()
    }
}

impl From<AccountType> for u64 {
    fn from(value: AccountType) -> Self {
        value as u64
    }
}

// ACCOUNT STORAGE TYPE
// ================================================================================================

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum AccountStorageType {
    OnChain = ACCOUNT_ON_CHAIN,
    OffChain = ACCOUNT_OFF_CHAIN,
}

impl Display for AccountStorageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountStorageType::OnChain => write!(f, "on-chain"),
            AccountStorageType::OffChain => write!(f, "off-chain"),
        }
    }
}

impl TryFrom<u64> for AccountStorageType {
    type Error = AccountError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            ACCOUNT_ON_CHAIN => Ok(AccountStorageType::OnChain),
            ACCOUNT_OFF_CHAIN => Ok(AccountStorageType::OffChain),
            v => Err(AccountError::InvalidStorageType(v)),
        }
    }
}

impl TryFrom<Felt> for AccountStorageType {
    type Error = AccountError;

    fn try_from(value: Felt) -> Result<Self, Self::Error> {
        value.as_int().try_into()
    }
}

impl From<AccountStorageType> for u64 {
    fn from(value: AccountStorageType) -> Self {
        value as u64
    }
}

// ACCOUNT POW
// ================================================================================================

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountPoW(u8);

impl AccountPoW {
    pub fn new(pow: u8) -> Result<Self, AccountError> {
        if u64::from(pow) > ACCOUNT_POW_MAXIMUM {
            Err(AccountError::InvalidPoW(u64::from(pow)))
        } else {
            Ok(AccountPoW(pow))
        }
    }

    /// Returns the value configured for the proof-of-work.
    pub const fn as_int(&self) -> u8 {
        self.0
    }
}

impl Display for AccountPoW {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AccountPow({})", self.0)
    }
}

impl TryFrom<u64> for AccountPoW {
    type Error = AccountError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let pow_bits = value & ACCOUNT_POW_MASK;
        if (value ^ pow_bits) != 0 {
            return Err(AccountError::InvalidPoW(value));
        }

        let pow = pow_bits >> ACCOUNT_POW_MASK_SHIFT;
        Ok(AccountPoW(
            pow.try_into()
                .expect("Expected the maximum value for PoW to be 64, which fits in a u8"),
        ))
    }
}

impl TryFrom<Felt> for AccountPoW {
    type Error = AccountError;

    fn try_from(value: Felt) -> Result<Self, Self::Error> {
        value.as_int().try_into()
    }
}

impl From<AccountPoW> for u64 {
    fn from(value: AccountPoW) -> u64 {
        (value.0 as u64) << ACCOUNT_POW_MASK_SHIFT
    }
}

// ACCOUNT CONFIG
// ================================================================================================

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountConfig(u64);

impl AccountConfig {
    pub fn new(account_type: AccountType, storage_type: AccountStorageType) -> Self {
        let pow: AccountPoW = account_type.required_pow();
        Self(u64::from(account_type) | u64::from(storage_type) | u64::from(pow))
    }

    pub fn new_with_pow(
        account_type: AccountType,
        storage_type: AccountStorageType,
        pow: AccountPoW,
    ) -> Result<Self, AccountError> {
        let pow = u64::from(pow);
        if u64::from(account_type.required_pow()) > pow {
            return Err(AccountError::InvalidPoW(pow));
        }

        Ok(Self(u64::from(account_type) | u64::from(storage_type) | pow))
    }

    /// Returns the configured [AccountType].
    pub fn account_type(&self) -> AccountType {
        (self.0 & ACCOUNT_TYPE_MASK).try_into().unwrap()
    }

    /// Returns the configured [AccountStorageType].
    pub fn storage_type(&self) -> AccountStorageType {
        (self.0 & ACCOUNT_STORAGE_MASK).try_into().unwrap()
    }

    /// Returns the configured [AccountPoW].
    pub fn pow(&self) -> AccountPoW {
        (self.0 & ACCOUNT_POW_MASK).try_into().unwrap()
    }
}

impl Display for AccountConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.account_type(), self.storage_type(), self.pow())
    }
}

impl TryFrom<u64> for AccountConfig {
    type Error = AccountError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let config_bits = value & ACCOUNT_CONFIG_MASK;
        if (value ^ config_bits) != 0 {
            return Err(AccountError::InvalidConfig(value));
        }

        Ok(AccountConfig(value))
    }
}

impl TryFrom<Felt> for AccountConfig {
    type Error = AccountError;

    fn try_from(value: Felt) -> Result<Self, Self::Error> {
        value.as_int().try_into()
    }
}

impl From<AccountConfig> for u64 {
    fn from(value: AccountConfig) -> Self {
        value.0
    }
}

impl From<AccountConfig> for Felt {
    fn from(value: AccountConfig) -> Self {
        Felt::try_from(value.0).expect("Account config should always fit into a Felt")
    }
}

// ACCOUNT ID
// ================================================================================================

/// Unique identifier of an account.
///
/// Account ID consists of 1 field element (~64 bits). The most significant bits in the id are used
/// to encode the account' storage and type.
///
/// The top two bits are used to encode the storage type. The values [ACCOUNT_OFF_CHAIN] and [ACCOUNT_ON_CHAIN]
/// encode the account's storage type. The next two bits encode the account type. The values
/// [FUNGIBLE_FAUCET], [NON_FUNGIBLE_FAUCET], [REGULAR_ACCOUNT_IMMUTABLE_CODE], and
/// [REGULAR_ACCOUNT_UPDATABLE_CODE] encode the account's type.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct AccountId(Felt);

impl AccountId {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns [AccountId] derived from the specified `seed`, `config`, `code root`, and `storage root`.
    ///
    /// The account id is defined as the first element of the following hash:
    ///
    /// > hash(SEED || CODE_ROOT || STORAGE_ROOT || [0,0,0,0])
    ///
    /// With the `config` overwritten on top of it.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// - The amount of PoW in the third element of the digest is not sufficient.
    pub fn new(
        seed: Word,
        config: AccountConfig,
        code_root: Digest,
        storage_root: Digest,
    ) -> Result<Self, AccountError> {
        let digest = compute_digest(seed, code_root, storage_root);

        Self::from_digest(&digest, config)
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns this account's [AccountType].
    pub fn account_type(&self) -> AccountType {
        AccountType::from(self)
    }

    /// Returns this account's [AccountConfig].
    pub fn config(&self) -> AccountConfig {
        AccountConfig(self.0.as_int() & ACCOUNT_CONFIG_MASK)
    }

    /// Returns true if this id is a faucet, i.e. can issue assets.
    pub fn is_faucet(&self) -> bool {
        matches!(
            self.account_type(),
            AccountType::FungibleFaucet | AccountType::NonFungibleFaucet
        )
    }

    /// Returns true if this id is a regular account, i.e. not a faucet.
    pub fn is_regular_account(&self) -> bool {
        matches!(
            self.account_type(),
            AccountType::RegularAccountUpdatableCode | AccountType::RegularAccountImmutableCode
        )
    }

    /// Returns this account's [AccountStorageType].
    pub fn storage_type(&self) -> AccountStorageType {
        AccountStorageType::from(self)
    }

    /// Returns true if this account storage is [AccountStorageType::OnChain].
    pub fn is_on_chain(&self) -> bool {
        self.storage_type() == AccountStorageType::OnChain
    }

    /// Finds and returns a seed suitable for creating an account ID for the specified account type
    /// using the provided initial seed as a starting point.
    pub fn get_account_seed(
        init_seed: [u8; 32],
        config: AccountConfig,
        code_root: Digest,
        storage_root: Digest,
    ) -> Result<Word, AccountError> {
        get_account_seed(init_seed, config, code_root, storage_root)
    }

    /// Creates an Account Id from a hex string. Assumes the string starts with "0x" and
    /// that the hexadecimal characters are big-endian encoded.
    pub fn from_hex(hex_value: &str) -> Result<AccountId, AccountError> {
        hex_to_bytes(hex_value)
            .map_err(|err| AccountError::HexParseError(err.to_string()))
            .and_then(|mut bytes: [u8; 8]| {
                // `bytes` ends up being parsed as felt, and the input to that is assumed to be little-endian
                // so we need to reverse the order
                bytes.reverse();
                bytes.try_into()
            })
    }

    /// Returns this account's hex representation.
    pub fn to_hex(&self) -> String {
        format!("0x{:016x}", self.0.as_int())
    }

    // UTILITY METHODS
    // --------------------------------------------------------------------------------------------

    /// Constructs an [AccountId] from the given [Digest].
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// - The amount of PoW in the third element of the digest is not sufficient.
    pub fn from_digest(digest: &Digest, config: AccountConfig) -> Result<AccountId, AccountError> {
        let configured_pow = u32::from(config.pow().as_int());
        let computed_pow = digest_pow(*digest);
        if configured_pow > computed_pow {
            return Err(AccountError::SeedDigestTooFewTrailingZeros {
                expected: configured_pow,
                actual: computed_pow,
            });
        }

        // The config must be grinded into the account id. This ensure the account id is a valid
        // field element and reduces the validation in the kernel to a single felt comparison
        let expected = u64::from(config);
        let actual = digest[0].as_int() & ACCOUNT_NON_RANDOM_MASK;
        if actual != expected {
            return Err(AccountError::ConfigDoesNotMatch(actual, expected));
        }

        Ok(AccountId(digest[0]))
    }
}

impl PartialOrd for AccountId {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AccountId {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.as_int().cmp(&other.0.as_int())
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:016x}", self.0.as_int())
    }
}

// CONVERSIONS FROM ACCOUNT ID
// ================================================================================================

impl From<&AccountId> for Felt {
    fn from(value: &AccountId) -> Self {
        (*value).into()
    }
}

impl From<AccountId> for Felt {
    fn from(value: AccountId) -> Self {
        value.0
    }
}

impl From<&AccountId> for AccountStorageType {
    fn from(value: &AccountId) -> Self {
        (*value).into()
    }
}

impl From<AccountId> for AccountStorageType {
    fn from(value: AccountId) -> Self {
        (value.0.as_int() & ACCOUNT_STORAGE_MASK)
            .try_into()
            .expect("Account constructed with invalid storage type")
    }
}

impl From<&AccountId> for AccountType {
    fn from(value: &AccountId) -> Self {
        (*value).into()
    }
}

impl From<AccountId> for AccountType {
    fn from(value: AccountId) -> Self {
        (value.0.as_int() & ACCOUNT_TYPE_MASK)
            .try_into()
            .expect("Account constructed with invalid type")
    }
}

impl From<&AccountId> for AccountPoW {
    fn from(value: &AccountId) -> Self {
        (*value).into()
    }
}

impl From<AccountId> for AccountPoW {
    fn from(value: AccountId) -> Self {
        (value.0.as_int() & ACCOUNT_POW_MASK)
            .try_into()
            .expect("Account constructed with invalid pow")
    }
}

impl From<&AccountId> for [u8; 8] {
    fn from(value: &AccountId) -> Self {
        (*value).into()
    }
}

impl From<AccountId> for [u8; 8] {
    fn from(value: AccountId) -> Self {
        let mut result = [0_u8; 8];
        result[..8].copy_from_slice(&value.0.as_int().to_le_bytes());
        result
    }
}

impl From<&AccountId> for u64 {
    fn from(value: &AccountId) -> Self {
        (*value).into()
    }
}

impl From<AccountId> for u64 {
    fn from(value: AccountId) -> Self {
        value.0.as_int()
    }
}

/// Account IDs are used as indexes in the account database, which is a tree of depth 64.
impl From<AccountId> for LeafIndex<ACCOUNT_TREE_DEPTH> {
    fn from(value: AccountId) -> Self {
        LeafIndex::new_max_depth(value.0.as_int())
    }
}

// CONVERSIONS TO ACCOUNT ID
// ================================================================================================

impl TryFrom<Felt> for AccountId {
    type Error = AccountError;

    fn try_from(value: Felt) -> Result<Self, Self::Error> {
        value.as_int().try_into()
    }
}

impl TryFrom<[u8; 8]> for AccountId {
    type Error = AccountError;

    // Expects little-endian byte order
    fn try_from(value: [u8; 8]) -> Result<Self, Self::Error> {
        let element = parse_felt(&value[..8])?;
        Self::try_from(element)
    }
}

impl TryFrom<u64> for AccountId {
    type Error = AccountError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let _ = AccountType::try_from(value & ACCOUNT_TYPE_MASK)?;
        let _ = AccountStorageType::try_from(value & ACCOUNT_STORAGE_MASK)?;
        let _ = AccountPoW::try_from(value & ACCOUNT_POW_MASK)?;

        let zeros = value & ACCOUNT_ZERO_MASK;
        if zeros != 0 {
            return Err(AccountError::IdMissingZeros(value));
        }

        let felt = Felt::try_from(value).map_err(|_| AccountError::InvalidFelt(value))?;
        Ok(Self(felt))
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountId {
    fn write_into<W: miden_crypto::utils::ByteWriter>(&self, target: &mut W) {
        self.0.write_into(target);
    }
}

impl Deserializable for AccountId {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        Felt::read_from(source)?
            .try_into()
            .map_err(|err: AccountError| DeserializationError::InvalidValue(err.to_string()))
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn parse_felt(bytes: &[u8]) -> Result<Felt, AccountError> {
    Felt::try_from(bytes).map_err(|err| AccountError::AccountIdInvalidFieldElement(err.to_string()))
}

/// Returns the account digest.
///
/// The digest is computed as:
///
/// > hash(SEED || CODE_ROOT || STORAGE_ROOT || [0,0,0,0])
///
pub fn compute_digest(seed: Word, code_root: Digest, storage_root: Digest) -> Digest {
    let mut elements = Vec::with_capacity(16);
    elements.extend(seed);
    elements.extend(code_root);
    elements.extend(storage_root);
    elements.extend([ZERO, ZERO, ZERO, ZERO]);
    Hasher::hash_elements(&elements)
}

/// Given a [Digest] returns its proof-of-work.
pub(super) fn digest_pow(digest: Digest) -> u32 {
    digest.as_elements()[3].as_int().trailing_zeros()
}

// TESTING
// ================================================================================================

#[cfg(any(feature = "testing", test))]
pub mod testing {
    use super::{
        AccountStorageType, AccountType, ACCOUNT_POW_MASK_SHIFT, ACCOUNT_RANDOM_BITS_MASK_SHIFT,
    };

    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    // REGULAR ACCOUNTS - OFF-CHAIN
    pub const ACCOUNT_ID_SENDER: u64 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageType::OffChain,
        0b0001_1111,
    );
    pub const ACCOUNT_ID_OFF_CHAIN_SENDER: u64 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageType::OffChain,
        0b0010_1111,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN: u64 = account_id(
        AccountType::RegularAccountUpdatableCode,
        AccountStorageType::OffChain,
        0b0011_1111,
    );
    // REGULAR ACCOUNTS - ON-CHAIN
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN: u64 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageType::OnChain,
        0b0001_1111,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2: u64 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageType::OnChain,
        0b0010_1111,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: u64 = account_id(
        AccountType::RegularAccountUpdatableCode,
        AccountStorageType::OnChain,
        0b0011_1111,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2: u64 = account_id(
        AccountType::RegularAccountUpdatableCode,
        AccountStorageType::OnChain,
        0b0100_1111,
    );

    // FUNGIBLE TOKENS - OFF-CHAIN
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageType::OffChain, 0b0001_1111);
    // FUNGIBLE TOKENS - ON-CHAIN
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageType::OnChain, 0b0001_1111);
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageType::OnChain, 0b0010_1111);
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageType::OnChain, 0b0011_1111);
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageType::OnChain, 0b0100_1111);

    // NON-FUNGIBLE TOKENS - OFF-CHAIN
    pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN: u64 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageType::OffChain, 0b0001_1111);
    // NON-FUNGIBLE TOKENS - ON-CHAIN
    pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN: u64 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageType::OnChain, 0b0010_1111);
    pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1: u64 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageType::OnChain, 0b0011_1111);

    // INVALID IDS
    pub const INVALID_ACCOUNT_ID_1: u64 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageType::OffChain, 0) | 0b1111;
    pub const INVALID_ACCOUNT_ID_2: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageType::OffChain, 0) | 0b1111;
    pub const INVALID_ACCOUNT_ID_3: u64 =
        account_id(AccountType::RegularAccountImmutableCode, AccountStorageType::OffChain, 0)
            | 0b1111;
    pub const INVALID_ACCOUNT_ID_4: u64 =
        account_id(AccountType::RegularAccountUpdatableCode, AccountStorageType::OffChain, 0)
            | 0b1111;
    pub const INVALID_ACCOUNT_ID_5: u64 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageType::OnChain, 0) | 0b1111;
    pub const INVALID_ACCOUNT_ID_6: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageType::OnChain, 0) | 0b1111;
    pub const INVALID_ACCOUNT_ID_7: u64 =
        account_id(AccountType::RegularAccountImmutableCode, AccountStorageType::OnChain, 0)
            | 0b1111;
    pub const INVALID_ACCOUNT_ID_8: u64 =
        account_id(AccountType::RegularAccountUpdatableCode, AccountStorageType::OnChain, 0)
            | 0b1111;

    // UTILITIES
    // --------------------------------------------------------------------------------------------

    pub const fn account_id(
        account_type: AccountType,
        storage: AccountStorageType,
        rest: u64,
    ) -> u64 {
        let mut id = 0;

        id ^= storage as u64;
        id ^= account_type as u64;
        id ^= rest << ACCOUNT_RANDOM_BITS_MASK_SHIFT;
        id ^= (account_type.required_pow().as_int() as u64) << ACCOUNT_POW_MASK_SHIFT;

        id
    }
}

// TESTS
// ================================================================================================
#[cfg(test)]
mod tests {
    use miden_crypto::utils::{Deserializable, Serializable};

    use super::{
        testing::*, AccountConfig, AccountId, AccountStorageType, AccountType,
        ACCOUNT_ISFAUCET_BIT, ACCOUNT_OFF_CHAIN, ACCOUNT_ON_CHAIN, ACCOUNT_POW_MASK,
        ACCOUNT_POW_MASK_SHIFT, ACCOUNT_RANDOM_BITS_MASK, ACCOUNT_RANDOM_BITS_MASK_SHIFT,
        ACCOUNT_STORAGE_MASK, ACCOUNT_STORAGE_MASK_SHIFT, ACCOUNT_TYPE_MASK,
        ACCOUNT_TYPE_MASK_SHIFT, ACCOUNT_ZERO_MASK, FUNGIBLE_FAUCET, NON_FUNGIBLE_FAUCET,
        REGULAR_ACCOUNT_IMMUTABLE_CODE, REGULAR_ACCOUNT_UPDATABLE_CODE,
    };

    #[test]
    fn test_account_id() {
        for account_type in [
            AccountType::RegularAccountImmutableCode,
            AccountType::RegularAccountUpdatableCode,
            AccountType::NonFungibleFaucet,
            AccountType::FungibleFaucet,
        ] {
            for storage_type in [AccountStorageType::OnChain, AccountStorageType::OffChain] {
                let config = AccountConfig::new(account_type, storage_type);
                assert_eq!(config.account_type(), account_type);
                assert_eq!(config.storage_type(), storage_type);
                assert_eq!(config.pow(), account_type.required_pow());

                let acc = AccountId::try_from(account_id(account_type, storage_type, 0b1111_1111))
                    .unwrap();

                assert_eq!(acc.account_type(), account_type);
                assert_eq!(acc.storage_type(), storage_type);
            }
        }
    }

    #[test]
    fn test_account_id_from_hex_and_back() {
        for account_id in [
            ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ] {
            let acc = AccountId::try_from(account_id).expect("Valid account ID");
            assert_eq!(acc, AccountId::from_hex(&acc.to_hex()).unwrap());
        }
    }

    #[test]
    fn test_account_id_serde() {
        let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN)
            .expect("Valid account ID");
        assert_eq!(account_id, AccountId::read_from_bytes(&account_id.to_bytes()).unwrap());

        let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN)
            .expect("Valid account ID");
        assert_eq!(account_id, AccountId::read_from_bytes(&account_id.to_bytes()).unwrap());

        let account_id =
            AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).expect("Valid account ID");
        assert_eq!(account_id, AccountId::read_from_bytes(&account_id.to_bytes()).unwrap());

        let account_id = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN)
            .expect("Valid account ID");
        assert_eq!(account_id, AccountId::read_from_bytes(&account_id.to_bytes()).unwrap());
    }

    #[test]
    fn test_account_id_account_type() {
        let account_id: AccountId =
            AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap();
        let account_type: AccountType = REGULAR_ACCOUNT_IMMUTABLE_CODE.try_into().unwrap();
        assert_eq!(account_type, account_id.account_type());

        let account_id =
            AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN).unwrap();
        let account_type: AccountType = REGULAR_ACCOUNT_UPDATABLE_CODE.try_into().unwrap();
        assert_eq!(account_type, account_id.account_type());

        let account_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
        let account_type: AccountType = FUNGIBLE_FAUCET.try_into().unwrap();
        assert_eq!(account_type, account_id.account_type());

        let account_id = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN).unwrap();
        let account_type: AccountType = NON_FUNGIBLE_FAUCET.try_into().unwrap();
        assert_eq!(account_type, account_id.account_type());
    }

    #[test]
    fn test_account_id_tag_identifiers() {
        let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN)
            .expect("Valid account ID");
        assert!(account_id.is_regular_account());
        assert_eq!(account_id.account_type(), AccountType::RegularAccountImmutableCode);
        assert!(account_id.is_on_chain());

        let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN)
            .expect("Valid account ID");
        assert!(account_id.is_regular_account());
        assert_eq!(account_id.account_type(), AccountType::RegularAccountUpdatableCode);
        assert!(!account_id.is_on_chain());

        let account_id =
            AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).expect("Valid account ID");
        assert!(account_id.is_faucet());
        assert_eq!(account_id.account_type(), AccountType::FungibleFaucet);
        assert!(account_id.is_on_chain());

        let account_id = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN)
            .expect("Valid account ID");
        assert!(account_id.is_faucet());
        assert_eq!(account_id.account_type(), AccountType::NonFungibleFaucet);
        assert!(!account_id.is_on_chain());
    }

    /// The following test ensure there is a bit available to identify an account as a faucet or
    /// normal.
    #[test]
    fn test_account_id_faucet_bit() {
        // faucets have a bit set
        assert_ne!(FUNGIBLE_FAUCET & ACCOUNT_ISFAUCET_BIT, 0);
        assert_ne!(NON_FUNGIBLE_FAUCET & ACCOUNT_ISFAUCET_BIT, 0);

        // normal accounts do not have the faucet bit set
        assert_eq!((REGULAR_ACCOUNT_IMMUTABLE_CODE) & ACCOUNT_ISFAUCET_BIT, 0);
        assert_eq!((REGULAR_ACCOUNT_UPDATABLE_CODE) & ACCOUNT_ISFAUCET_BIT, 0);
    }

    /// Every bit must be covered by at least one mask
    #[test]
    fn test_masks_cover_u64() {
        let masks = ACCOUNT_STORAGE_MASK
            | ACCOUNT_TYPE_MASK
            | ACCOUNT_POW_MASK
            | ACCOUNT_RANDOM_BITS_MASK
            | ACCOUNT_ZERO_MASK;
        assert_eq!(masks, u64::MAX);
    }

    /// Every bit must be covered by at most one mask, i.e. each bit has exactly one interpretation
    #[test]
    fn test_masks_dont_overlap() {
        let masks = [
            ACCOUNT_STORAGE_MASK,
            ACCOUNT_TYPE_MASK,
            ACCOUNT_POW_MASK,
            ACCOUNT_RANDOM_BITS_MASK,
            ACCOUNT_ZERO_MASK,
        ];

        for (i, left) in masks.iter().enumerate() {
            for (j, right) in masks.iter().enumerate() {
                if i == j {
                    continue;
                }

                let overlap = left & right;
                assert_eq!(overlap, 0);
            }
        }
    }

    /// Make sure the shift matches the masks
    #[test]
    fn test_shifts_match_mask_start() {
        let mask_shift = [
            (ACCOUNT_STORAGE_MASK, ACCOUNT_STORAGE_MASK_SHIFT),
            (ACCOUNT_TYPE_MASK, ACCOUNT_TYPE_MASK_SHIFT),
            (ACCOUNT_POW_MASK, ACCOUNT_POW_MASK_SHIFT),
            (ACCOUNT_RANDOM_BITS_MASK, ACCOUNT_RANDOM_BITS_MASK_SHIFT),
            (ACCOUNT_ZERO_MASK, 0),
        ];

        for (mask, shift) in mask_shift {
            assert_eq!(u64::from(mask.trailing_zeros()), shift);
        }
    }

    /// Make sure the defaults match the masks
    #[test]
    fn test_defaults_are_inside_valid_range() {
        let default_mask = [
            (ACCOUNT_ISFAUCET_BIT, ACCOUNT_TYPE_MASK),
            (FUNGIBLE_FAUCET, ACCOUNT_TYPE_MASK),
            (NON_FUNGIBLE_FAUCET, ACCOUNT_TYPE_MASK),
            (REGULAR_ACCOUNT_IMMUTABLE_CODE, ACCOUNT_TYPE_MASK),
            (REGULAR_ACCOUNT_UPDATABLE_CODE, ACCOUNT_TYPE_MASK),
            (ACCOUNT_ON_CHAIN, ACCOUNT_STORAGE_MASK),
            (ACCOUNT_OFF_CHAIN, ACCOUNT_STORAGE_MASK),
        ];

        for (default, mask) in default_mask {
            let masked = default & mask;
            assert_eq!(default, masked);
        }
    }
}
