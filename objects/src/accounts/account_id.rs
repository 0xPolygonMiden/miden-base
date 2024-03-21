use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use super::{
    get_account_seed, Account, AccountError, ByteReader, Deserializable, DeserializationError,
    Digest, Felt, FieldElement, Hasher, Serializable, Word,
};
use crate::{crypto::merkle::LeafIndex, utils::hex_to_bytes, ACCOUNT_TREE_DEPTH};

// ACCOUNT ID
// ================================================================================================

/// Specifies the account type.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountType {
    FungibleFaucet,
    NonFungibleFaucet,
    RegularAccountImmutableCode,
    RegularAccountUpdatableCode,
}

/// Unique identifier of an account.
///
/// Account ID consists of 1 field element (~64 bits). This field element uniquely identifies a
/// single account and also specifies the type of the underlying account. Specifically:
/// - The two most significant bits of the ID specify the type of the account:
///  - 00 - regular account with updatable code.
///  - 01 - regular account with immutable code.
///  - 10 - fungible asset faucet with immutable code.
///  - 11 - non-fungible asset faucet with immutable code.
/// - The third most significant bit of the ID specifies whether the account data is stored on-chain:
///  - 0 - full account data is stored on-chain.
///  - 1 - only the account hash is stored on-chain which serves as a commitment to the account state.
/// As such the three most significant bits fully describes the type of the account.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct AccountId(Felt);

impl AccountId {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------
    pub const FUNGIBLE_FAUCET_TAG: u64 = 0b10;
    pub const NON_FUNGIBLE_FAUCET_TAG: u64 = 0b11;
    pub const REGULAR_ACCOUNT_UPDATABLE_CODE_TAG: u64 = 0b00;
    pub const REGULAR_ACCOUNT_IMMUTABLE_CODE_TAG: u64 = 0b01;
    pub const ON_CHAIN_ACCOUNT_SELECTOR: u64 = 0b001;

    /// Specifies a minimum number of trailing zeros required in the last element of the seed digest.
    ///
    /// Note: The account id includes 3 bits of metadata, these bits determine the account type
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

    /// Returns a new account ID derived from the specified seed, code root and storage root.
    ///
    /// The account ID is computed by hashing the seed, code root and storage root and using 1
    /// element of the resulting digest to form the ID. Specifically we take element 0. We also
    /// require that the last element of the seed digest has at least `23` trailing zeros if it
    /// is a regular account, or `31` trailing zeros if it is a faucet account.
    ///
    /// The seed digest is computed using a sequential hash over
    /// hash(SEED, CODE_ROOT, STORAGE_ROOT, ZERO).  This takes two permutations.
    ///
    /// # Errors
    /// Returns an error if the resulting account ID does not comply with account ID rules:
    /// - the ID has at least `5` ones.
    /// - the ID has at least `23` trailing zeros if it is a regular account.
    /// - the ID has at least `31` trailing zeros if it is a faucet account.
    pub fn new(seed: Word, code_root: Digest, storage_root: Digest) -> Result<Self, AccountError> {
        let seed_digest = compute_digest(seed, code_root, storage_root);

        // verify the seed digest satisfies all rules
        Self::validate_seed_digest(&seed_digest)?;

        // construct the ID from the first element of the seed hash
        let id = Self(seed_digest[0]);

        Ok(id)
    }

    /// Creates a new [AccountId] without checking its validity.
    ///
    /// This function requires that the provided value is a valid [Felt] representation of an
    /// [AccountId].
    pub fn new_unchecked(value: Felt) -> AccountId {
        AccountId(value)
    }

    /// Creates a new dummy [AccountId] for testing purposes.
    #[cfg(any(feature = "testing", test))]
    pub fn new_dummy(init_seed: [u8; 32], account_type: AccountType) -> Self {
        let code_root = Digest::default();
        let storage_root = Digest::default();

        let seed =
            get_account_seed(init_seed, account_type, true, code_root, storage_root).unwrap();

        AccountId::new(seed, code_root, storage_root).unwrap()
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the type of this account ID.
    pub fn account_type(&self) -> AccountType {
        match self.0.as_int() >> 62 {
            Self::REGULAR_ACCOUNT_UPDATABLE_CODE_TAG => AccountType::RegularAccountUpdatableCode,
            Self::REGULAR_ACCOUNT_IMMUTABLE_CODE_TAG => AccountType::RegularAccountImmutableCode,
            Self::FUNGIBLE_FAUCET_TAG => AccountType::FungibleFaucet,
            Self::NON_FUNGIBLE_FAUCET_TAG => AccountType::NonFungibleFaucet,
            _ => unreachable!(),
        }
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
        matches!(
            self.account_type(),
            AccountType::RegularAccountUpdatableCode | AccountType::RegularAccountImmutableCode
        )
    }

    /// Returns true if an account with this ID is an on-chain account.
    pub fn is_on_chain(&self) -> bool {
        self.0.as_int() >> 61 & Self::ON_CHAIN_ACCOUNT_SELECTOR == 1
    }

    /// Finds and returns a seed suitable for creating an account ID for the specified account type
    /// using the provided initial seed as a starting point.
    pub fn get_account_seed(
        init_seed: [u8; 32],
        account_type: AccountType,
        on_chain: bool,
        code_root: Digest,
        storage_root: Digest,
    ) -> Result<Word, AccountError> {
        get_account_seed(init_seed, account_type, on_chain, code_root, storage_root)
    }

    /// Returns an error if:
    /// - There are fewer then:
    ///     - 24 trailing ZEROs in the last element of the seed digest for regular accounts.
    ///     - 32 trailing ZEROs in the last element of the seed digest for faucet accounts.
    /// - There are fewer than 5 ONEs in the account ID (first element of the seed digest).
    pub fn validate_seed_digest(digest: &Digest) -> Result<(), AccountError> {
        let elements = digest.as_elements();

        // accounts must have at least 5 ONEs in the ID.
        if elements[0].as_int().count_ones() < Self::MIN_ACCOUNT_ONES {
            return Err(AccountError::account_id_too_few_ones());
        }

        // we require that accounts have at least some number of trailing zeros in the last element,
        let is_regular_account = elements[0].as_int() >> 63 == 0;
        let pow_trailing_zeros = digest_pow(*digest);

        // check if there is there enough trailing zeros in the last element of the seed hash for
        // the account type.
        let expected = match is_regular_account {
            true => Self::REGULAR_ACCOUNT_SEED_DIGEST_MIN_TRAILING_ZEROS,
            false => Self::FAUCET_SEED_DIGEST_MIN_TRAILING_ZEROS,
        };
        let sufficient_pow = pow_trailing_zeros >= expected;

        if !sufficient_pow {
            return Err(AccountError::seed_digest_too_few_trailing_zeros(
                expected,
                pow_trailing_zeros,
            ));
        }

        Ok(())
    }

    /// Returns an error if:
    /// - There are fewer then 5 ONEs in the account ID.
    fn validate(&self) -> Result<(), AccountError> {
        if self.0.as_int().count_ones() < Self::MIN_ACCOUNT_ONES {
            return Err(AccountError::account_id_too_few_ones());
        }

        Ok(())
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

    /// Returns a big-endian, hex-encoded string.
    pub fn to_hex(&self) -> String {
        format!("0x{:02x}", self.0.as_int())
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
        write!(f, "0x{:02x}", self.0.as_int())
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

impl TryFrom<Felt> for AccountId {
    type Error = AccountError;

    fn try_from(value: Felt) -> Result<Self, Self::Error> {
        let id = Self(value);
        id.validate()?;
        Ok(id)
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
}

impl Deserializable for AccountId {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        Ok(AccountId(Felt::read_from(source)?))
    }
}

// HELPER FUNCTIONS
// ================================================================================================
fn parse_felt(bytes: &[u8]) -> Result<Felt, AccountError> {
    Felt::try_from(bytes).map_err(|err| AccountError::AccountIdInvalidFieldElement(err.to_string()))
}

/// Validates that the provided seed is valid for the provided account.
pub fn validate_account_seed(account: &Account, seed: Word) -> Result<(), AccountError> {
    let account_id = AccountId::new(seed, account.code().root(), account.storage().root())?;
    if account_id != account.id() {
        return Err(AccountError::InconsistentAccountIdSeed {
            expected: account.id(),
            actual: account_id,
        });
    }

    Ok(())
}

/// Returns the digest of two hashing permutations over the seed, code root, storage root and
/// padding.
pub fn compute_digest(seed: Word, code_root: Digest, storage_root: Digest) -> Digest {
    let mut elements = Vec::with_capacity(16);
    elements.extend(seed);
    elements.extend(*code_root);
    elements.extend(*storage_root);
    elements.resize(16, Felt::ZERO);
    Hasher::hash_elements(&elements)
}

/// Given a [Digest] returns its proof-of-work.
pub fn digest_pow(digest: Digest) -> u32 {
    digest.as_elements()[3].as_int().trailing_zeros()
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::{
        super::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        },
        AccountId, AccountType,
    };

    #[test]
    fn test_from_hex_and_back() {
        let account_id_hex = "0x45ce97a017946317";
        let account_id = AccountId::from_hex(account_id_hex).unwrap();

        assert_eq!(account_id.to_hex(), account_id_hex);
    }

    #[test]
    fn test_account_tag_identifiers() {
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
}
