use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{fmt, str::FromStr};

use super::{
    get_account_seed, AccountError, ByteReader, Deserializable, DeserializationError, Digest, Felt,
    Hasher, Serializable, Word, ZERO,
};
use crate::{crypto::merkle::LeafIndex, utils::hex_to_bytes, ACCOUNT_TREE_DEPTH};

// CONSTANTS
// ================================================================================================

// The higher two bits of the most significant nibble determines the account storage mode
pub const ACCOUNT_STORAGE_MASK_SHIFT: u64 = 62;
pub const ACCOUNT_STORAGE_MASK: u64 = 0b11 << ACCOUNT_STORAGE_MASK_SHIFT;

// The lower two bits of the most significant nibble determines the account type
pub const ACCOUNT_TYPE_MASK_SHIFT: u64 = 60;
pub const ACCOUNT_TYPE_MASK: u64 = 0b11 << ACCOUNT_TYPE_MASK_SHIFT;
pub const ACCOUNT_ISFAUCET_MASK: u64 = 0b10 << ACCOUNT_TYPE_MASK_SHIFT;

// ACCOUNT TYPES
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

/// Extracts the [AccountType] encoded in an u64.
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

    let bits = (value & ACCOUNT_TYPE_MASK) >> ACCOUNT_TYPE_MASK_SHIFT;
    match bits {
        REGULAR_ACCOUNT_UPDATABLE_CODE => AccountType::RegularAccountUpdatableCode,
        REGULAR_ACCOUNT_IMMUTABLE_CODE => AccountType::RegularAccountImmutableCode,
        FUNGIBLE_FAUCET => AccountType::FungibleFaucet,
        NON_FUNGIBLE_FAUCET => AccountType::NonFungibleFaucet,
        _ => {
            // "account_type mask contains only 2bits, there are 4 options total"
            unreachable!()
        },
    }
}

/// Returns the [AccountType] given an integer representation of `account_id`.
impl From<u64> for AccountType {
    fn from(value: u64) -> Self {
        account_type_from_u64(value)
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
            _ => Err(AccountError::InvalidAccountStorageMode),
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

/// Unique identifier of an account.
///
/// Account ID consists of 1 field element (~64 bits). The most significant bits in the id are used
/// to encode the account' storage and type.
///
/// The top two bits are used to encode the storage type. The values [PRIVATE] and [PUBLIC]
/// encode the account's storage type. The next two bits encode the account type. The values
/// [FUNGIBLE_FAUCET], [NON_FUNGIBLE_FAUCET], [REGULAR_ACCOUNT_IMMUTABLE_CODE], and
/// [REGULAR_ACCOUNT_UPDATABLE_CODE] encode the account's type.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AccountId(Felt);

impl AccountId {
    /// Specifies a minimum number of trailing zeros required in the last element of the seed
    /// digest.
    ///
    /// Note: The account id includes 4 bits of metadata, these bits determine the account type
    /// (normal account, fungible token, non-fungible token), the storage type (on/off chain), and
    /// for the normal accounts if the code is updatable or not. These metadata bits are also
    /// checked by the PoW and add to the total work defined below.
    #[cfg(not(any(feature = "testing", test)))]
    pub const REGULAR_ACCOUNT_SEED_DIGEST_MIN_TRAILING_ZEROS: u32 = 23;
    #[cfg(not(any(feature = "testing", test)))]
    pub const FAUCET_SEED_DIGEST_MIN_TRAILING_ZEROS: u32 = 31;
    #[cfg(any(feature = "testing", test))]
    pub const REGULAR_ACCOUNT_SEED_DIGEST_MIN_TRAILING_ZEROS: u32 = 5;
    #[cfg(any(feature = "testing", test))]
    pub const FAUCET_SEED_DIGEST_MIN_TRAILING_ZEROS: u32 = 6;

    /// Specifies a minimum number of ones for a valid account ID.
    pub const MIN_ACCOUNT_ONES: u32 = 5;

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new account ID derived from the specified seed, code commitment and storage
    /// commitment.
    ///
    /// The account ID is computed by hashing the seed, code commitment and storage commitment and
    /// using 1 element of the resulting digest to form the ID. Specifically we take element 0.
    /// We also require that the last element of the seed digest has at least `23` trailing
    /// zeros if it is a regular account, or `31` trailing zeros if it is a faucet account.
    ///
    /// The seed digest is computed using a sequential hash over
    /// hash(SEED, CODE_COMMITMENT, STORAGE_COMMITMENT, ZERO).  This takes two permutations.
    ///
    /// # Errors
    /// Returns an error if the resulting account ID does not comply with account ID rules:
    /// - the metadata embedded in the ID (i.e., the first 4 bits) is valid.
    /// - the ID has at least `5` ones.
    /// - the last element of the seed digest has at least `23` trailing zeros for regular accounts.
    /// - the last element of the seed digest has at least `31` trailing zeros for faucet accounts.
    pub fn new(
        seed: Word,
        code_commitment: Digest,
        storage_commitment: Digest,
    ) -> Result<Self, AccountError> {
        let seed_digest = compute_digest(seed, code_commitment, storage_commitment);

        Self::validate_seed_digest(&seed_digest)?;
        seed_digest[0].try_into()
    }

    /// Creates a new [AccountId] without checking its validity.
    ///
    /// This function requires that the provided value is a valid [Felt] representation of an
    /// [AccountId].
    pub fn new_unchecked(value: Felt) -> Self {
        Self(value)
    }

    /// Creates a new dummy [AccountId] for testing purposes.
    #[cfg(any(feature = "testing", test))]
    pub fn new_dummy(init_seed: [u8; 32], account_type: AccountType) -> Self {
        let code_commitment = Digest::default();
        let storage_commitment = Digest::default();

        let seed = get_account_seed(
            init_seed,
            account_type,
            AccountStorageMode::Public,
            code_commitment,
            storage_commitment,
        )
        .unwrap();

        Self::new(seed, code_commitment, storage_commitment).unwrap()
    }

    /// Constructs an [`AccountId`] for testing purposes with the given account type and storage
    /// mode.
    ///
    /// This function does the following:
    /// - The bit representation of the account type and storage mode is prepended to the most
    ///   significant byte of `bytes`.
    /// - The 5th most significant bit is cleared.
    /// - The bytes are then converted to a `u64` in big-endian format. Due to clearing the 5th most
    ///   significant bit, the resulting `u64` will be a valid [`Felt`].
    #[cfg(any(feature = "testing", test))]
    pub fn new_with_type_and_mode(
        mut bytes: [u8; 8],
        account_type: AccountType,
        storage_mode: AccountStorageMode,
    ) -> AccountId {
        let id_high_nibble = (storage_mode as u8) << 6 | (account_type as u8) << 4;

        // Clear the highest five bits of the most significant byte.
        // The high nibble must be cleared so we can set it to the storage mode and account type
        // we've constructed.
        // The 5th most significant bit is cleared to ensure the resulting id is a valid Felt even
        // when all other bits are set.
        bytes[0] &= 0x07;
        // Set high nibble of the most significant byte.
        bytes[0] |= id_high_nibble;

        let account_id = Felt::try_from(u64::from_be_bytes(bytes))
            .expect("must be a valid felt after clearing the 5th highest bit");
        let account_id = AccountId::new_unchecked(account_id);

        debug_assert_eq!(account_id.account_type(), account_type);
        debug_assert_eq!(account_id.storage_mode(), storage_mode);

        account_id
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the type of this account ID.
    pub const fn account_type(&self) -> AccountType {
        account_type_from_u64(self.0.as_int())
    }

    /// Returns true if an account with this ID is a faucet (can issue assets).
    pub fn is_faucet(&self) -> bool {
        matches!(
            self.account_type(),
            AccountType::FungibleFaucet | AccountType::NonFungibleFaucet
        )
    }

    /// Returns true if an account with this ID is a regular account.
    pub fn is_regular_account(&self) -> bool {
        is_regular_account(self.0.as_int())
    }

    /// Returns the storage mode of this account (e.g., public or private).
    pub fn storage_mode(&self) -> AccountStorageMode {
        let bits = (self.0.as_int() & ACCOUNT_STORAGE_MASK) >> ACCOUNT_STORAGE_MASK_SHIFT;
        match bits {
            PUBLIC => AccountStorageMode::Public,
            PRIVATE => AccountStorageMode::Private,
            _ => panic!("Account with invalid storage bits created"),
        }
    }

    /// Returns true if an account with this ID is a public account.
    pub fn is_public(&self) -> bool {
        self.storage_mode() == AccountStorageMode::Public
    }

    /// Finds and returns a seed suitable for creating an account ID for the specified account type
    /// using the provided initial seed as a starting point.
    pub fn get_account_seed(
        init_seed: [u8; 32],
        account_type: AccountType,
        storage_mode: AccountStorageMode,
        code_commitment: Digest,
        storage_commitment: Digest,
    ) -> Result<Word, AccountError> {
        get_account_seed(init_seed, account_type, storage_mode, code_commitment, storage_commitment)
    }

    /// Creates an Account Id from a hex string. Assumes the string starts with "0x" and
    /// that the hexadecimal characters are big-endian encoded.
    pub fn from_hex(hex_value: &str) -> Result<AccountId, AccountError> {
        hex_to_bytes(hex_value)
            .map_err(|err| AccountError::HexParseError(err.to_string()))
            .and_then(|mut bytes: [u8; 8]| {
                // `bytes` ends up being parsed as felt, and the input to that is assumed to be
                // little-endian so we need to reverse the order
                bytes.reverse();
                bytes.try_into()
            })
    }

    /// Returns a big-endian, hex-encoded string.
    pub fn to_hex(&self) -> String {
        format!("0x{:016x}", self.0.as_int())
    }

    // UTILITY METHODS
    // --------------------------------------------------------------------------------------------

    /// Returns an error if:
    /// - There are fewer then:
    ///   - 24 trailing ZEROs in the last element of the seed digest for regular accounts.
    ///   - 32 trailing ZEROs in the last element of the seed digest for faucet accounts.
    pub(super) fn validate_seed_digest(digest: &Digest) -> Result<(), AccountError> {
        // check the id satisfies the proof-of-work requirement.
        let required_zeros = if is_regular_account(digest[0].as_int()) {
            Self::REGULAR_ACCOUNT_SEED_DIGEST_MIN_TRAILING_ZEROS
        } else {
            Self::FAUCET_SEED_DIGEST_MIN_TRAILING_ZEROS
        };

        let trailing_zeros = digest_pow(*digest);
        if required_zeros > trailing_zeros {
            return Err(AccountError::SeedDigestTooFewTrailingZeros {
                expected: required_zeros,
                actual: trailing_zeros,
            });
        }

        Ok(())
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

impl From<AccountId> for Felt {
    fn from(id: AccountId) -> Self {
        id.0
    }
}

impl From<AccountId> for [u8; 8] {
    fn from(id: AccountId) -> Self {
        let mut result = [0_u8; 8];
        result[..8].copy_from_slice(&id.0.as_int().to_le_bytes());
        result
    }
}

impl From<AccountId> for u64 {
    fn from(id: AccountId) -> Self {
        id.0.as_int()
    }
}

/// Account IDs are used as indexes in the account database, which is a tree of depth 64.
impl From<AccountId> for LeafIndex<ACCOUNT_TREE_DEPTH> {
    fn from(id: AccountId) -> Self {
        LeafIndex::new_max_depth(id.0.as_int())
    }
}

// CONVERSIONS TO ACCOUNT ID
// ================================================================================================

/// Returns an [AccountId] instantiated with the provided field element.
///
/// # Errors
/// Returns an error if:
/// - If there are fewer than [AccountId::MIN_ACCOUNT_ONES] in the provided value.
/// - If the provided value contains invalid account ID metadata (i.e., the first 4 bits).
pub const fn account_id_from_felt(value: Felt) -> Result<AccountId, AccountError> {
    let int_value = value.as_int();

    let count = int_value.count_ones();
    if count < AccountId::MIN_ACCOUNT_ONES {
        return Err(AccountError::AccountIdTooFewOnes(AccountId::MIN_ACCOUNT_ONES, count));
    }

    let bits = (int_value & ACCOUNT_STORAGE_MASK) >> ACCOUNT_STORAGE_MASK_SHIFT;
    match bits {
        PUBLIC | PRIVATE => (),
        _ => return Err(AccountError::InvalidAccountStorageMode),
    };

    Ok(AccountId(value))
}

impl TryFrom<Felt> for AccountId {
    type Error = AccountError;

    /// Returns an [AccountId] instantiated with the provided field element.
    ///
    /// # Errors
    /// Returns an error if:
    /// - If there are fewer than [AccountId::MIN_ACCOUNT_ONES] in the provided value.
    /// - If the provided value contains invalid account ID metadata (i.e., the first 4 bits).
    fn try_from(value: Felt) -> Result<Self, Self::Error> {
        account_id_from_felt(value)
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
        let element = parse_felt(&value.to_le_bytes())?;
        Self::try_from(element)
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountId {
    fn write_into<W: miden_crypto::utils::ByteWriter>(&self, target: &mut W) {
        self.0.write_into(target);
    }

    fn get_size_hint(&self) -> usize {
        self.0.get_size_hint()
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

/// Returns the digest of two hashing permutations over the seed, code commitment, storage
/// commitment and padding.
pub(super) fn compute_digest(
    seed: Word,
    code_commitment: Digest,
    storage_commitment: Digest,
) -> Digest {
    let mut elements = Vec::with_capacity(16);
    elements.extend(seed);
    elements.extend(*code_commitment);
    elements.extend(*storage_commitment);
    elements.resize(16, ZERO);
    Hasher::hash_elements(&elements)
}

/// Given a [Digest] returns its proof-of-work.
pub(super) fn digest_pow(digest: Digest) -> u32 {
    digest.as_elements()[3].as_int().trailing_zeros()
}

/// Returns true if an account with this ID is a regular account.
fn is_regular_account(account_id: u64) -> bool {
    let account_type = account_id.into();
    matches!(
        account_type,
        AccountType::RegularAccountUpdatableCode | AccountType::RegularAccountImmutableCode
    )
}

// TESTING
// ================================================================================================

#[cfg(any(feature = "testing", test))]
pub mod testing {
    use super::{
        AccountStorageMode, AccountType, ACCOUNT_STORAGE_MASK_SHIFT, ACCOUNT_TYPE_MASK_SHIFT,
    };

    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    // REGULAR ACCOUNTS - OFF-CHAIN
    pub const ACCOUNT_ID_SENDER: u64 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageMode::Private,
        0b0001_1111,
    );
    pub const ACCOUNT_ID_OFF_CHAIN_SENDER: u64 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageMode::Private,
        0b0010_1111,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN: u64 = account_id(
        AccountType::RegularAccountUpdatableCode,
        AccountStorageMode::Private,
        0b0011_1111,
    );
    // REGULAR ACCOUNTS - ON-CHAIN
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN: u64 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageMode::Public,
        0b0001_1111,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2: u64 = account_id(
        AccountType::RegularAccountImmutableCode,
        AccountStorageMode::Public,
        0b0010_1111,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: u64 = account_id(
        AccountType::RegularAccountUpdatableCode,
        AccountStorageMode::Public,
        0b0011_1111,
    );
    pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2: u64 = account_id(
        AccountType::RegularAccountUpdatableCode,
        AccountStorageMode::Public,
        0b0100_1111,
    );

    // FUNGIBLE TOKENS - OFF-CHAIN
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageMode::Private, 0b0001_1111);
    // FUNGIBLE TOKENS - ON-CHAIN
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageMode::Public, 0b0001_1111);
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageMode::Public, 0b0010_1111);
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageMode::Public, 0b0011_1111);
    pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3: u64 =
        account_id(AccountType::FungibleFaucet, AccountStorageMode::Public, 0b0100_1111);

    // NON-FUNGIBLE TOKENS - OFF-CHAIN
    pub const ACCOUNT_ID_INSUFFICIENT_ONES: u64 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageMode::Private, 0b0000_0000); // invalid
    pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN: u64 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageMode::Private, 0b0001_1111);
    // NON-FUNGIBLE TOKENS - ON-CHAIN
    pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN: u64 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageMode::Public, 0b0010_1111);
    pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1: u64 =
        account_id(AccountType::NonFungibleFaucet, AccountStorageMode::Public, 0b0011_1111);

    // UTILITIES
    // --------------------------------------------------------------------------------------------

    pub const fn account_id(
        account_type: AccountType,
        storage_mode: AccountStorageMode,
        rest: u64,
    ) -> u64 {
        let mut id = 0;

        id ^= (storage_mode as u64) << ACCOUNT_STORAGE_MASK_SHIFT;
        id ^= (account_type as u64) << ACCOUNT_TYPE_MASK_SHIFT;
        id ^= rest;

        id
    }
}

// TESTS
// ================================================================================================
#[cfg(test)]
mod tests {
    use miden_crypto::utils::{Deserializable, Serializable};

    use super::{
        testing::*, AccountId, AccountStorageMode, AccountType, ACCOUNT_ISFAUCET_MASK,
        ACCOUNT_TYPE_MASK_SHIFT, FUNGIBLE_FAUCET, NON_FUNGIBLE_FAUCET,
        REGULAR_ACCOUNT_IMMUTABLE_CODE, REGULAR_ACCOUNT_UPDATABLE_CODE,
    };

    #[test]
    fn test_account_id() {
        use crate::accounts::AccountId;

        for account_type in [
            AccountType::RegularAccountImmutableCode,
            AccountType::RegularAccountUpdatableCode,
            AccountType::NonFungibleFaucet,
            AccountType::FungibleFaucet,
        ] {
            for storage_mode in [AccountStorageMode::Public, AccountStorageMode::Private] {
                let acc = AccountId::try_from(account_id(account_type, storage_mode, 0b1111_1111))
                    .unwrap();
                assert_eq!(acc.account_type(), account_type);
                assert_eq!(acc.storage_mode(), storage_mode);
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
        let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN)
            .expect("Valid account ID");

        let account_type: AccountType = ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.into();
        assert_eq!(account_type, account_id.account_type());

        let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN)
            .expect("Valid account ID");
        let account_type: AccountType = ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN.into();
        assert_eq!(account_type, account_id.account_type());

        let account_id =
            AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).expect("Valid account ID");
        let account_type: AccountType = ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.into();
        assert_eq!(account_type, account_id.account_type());

        let account_id = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN)
            .expect("Valid account ID");
        let account_type: AccountType = ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN.into();
        assert_eq!(account_type, account_id.account_type());
    }

    #[test]
    fn test_account_id_tag_identifiers() {
        let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN)
            .expect("Valid account ID");
        assert!(account_id.is_regular_account());
        assert_eq!(account_id.account_type(), AccountType::RegularAccountImmutableCode);
        assert!(account_id.is_public());

        let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN)
            .expect("Valid account ID");
        assert!(account_id.is_regular_account());
        assert_eq!(account_id.account_type(), AccountType::RegularAccountUpdatableCode);
        assert!(!account_id.is_public());

        let account_id =
            AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).expect("Valid account ID");
        assert!(account_id.is_faucet());
        assert_eq!(account_id.account_type(), AccountType::FungibleFaucet);
        assert!(account_id.is_public());

        let account_id = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN)
            .expect("Valid account ID");
        assert!(account_id.is_faucet());
        assert_eq!(account_id.account_type(), AccountType::NonFungibleFaucet);
        assert!(!account_id.is_public());
    }

    /// The following test ensure there is a bit available to identify an account as a faucet or
    /// normal.
    #[test]
    fn test_account_id_faucet_bit() {
        // faucets have a bit set
        assert_ne!((FUNGIBLE_FAUCET << ACCOUNT_TYPE_MASK_SHIFT) & ACCOUNT_ISFAUCET_MASK, 0);
        assert_ne!((NON_FUNGIBLE_FAUCET << ACCOUNT_TYPE_MASK_SHIFT) & ACCOUNT_ISFAUCET_MASK, 0);

        // normal accounts do not have the faucet bit set
        assert_eq!(
            (REGULAR_ACCOUNT_IMMUTABLE_CODE << ACCOUNT_TYPE_MASK_SHIFT) & ACCOUNT_ISFAUCET_MASK,
            0
        );
        assert_eq!(
            (REGULAR_ACCOUNT_UPDATABLE_CODE << ACCOUNT_TYPE_MASK_SHIFT) & ACCOUNT_ISFAUCET_MASK,
            0
        );
    }

    #[test]
    fn account_id_construction() {
        // Use the highest possible input to check if the constructed id is a valid Felt in that
        // scenario.
        let bytes = [0xff; 8];

        for account_type in [
            AccountType::FungibleFaucet,
            AccountType::NonFungibleFaucet,
            AccountType::RegularAccountImmutableCode,
            AccountType::RegularAccountUpdatableCode,
        ] {
            for storage_mode in [AccountStorageMode::Private, AccountStorageMode::Public] {
                // This function contains debug assertions already so we don't asset anything
                // additional
                AccountId::new_with_type_and_mode(bytes, account_type, storage_mode);
            }
        }
    }
}
