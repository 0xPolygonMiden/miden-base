use super::{AccountError, Digest, Felt, Hasher, StarkField, ToString, Vec, Word};
use core::{fmt, hash::Hash, ops::Deref};

// ACCOUNT ID
// ================================================================================================

/// Specifies the account type.
#[derive(Debug, PartialEq, Eq)]
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
    pub const REGULAR_ACCOUNT_SEED_DIGEST_MIN_TRAILING_ZEROS: u32 = 24;
    pub const FAUCET_SEED_DIGEST_MIN_TRAILING_ZEROS: u32 = 32;

    /// Specifies a minimum number of ones for a valid account ID.
    pub const MIN_ACCOUNT_ONES: u32 = 5;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new account ID derived from the specified seed.
    ///
    /// The account ID is computed by hashing the seed and using 1 element of the result to form
    /// the ID. Specifically we take element 0. We also require that the last element of the seed
    /// digest has at least `24` trailing zeros if it is a regular account, or `32` trailing zeros
    /// if it is a faucet account.
    ///
    /// # Errors
    /// Returns an error if the resulting account ID does not comply with account ID rules:
    /// - the ID has at least `5` ones.
    /// - the ID has at least `24` trailing zeros if it is a regular account.
    /// - the ID has at least `32` trailing zeros if it is a faucet account.
    pub fn new(seed: Word) -> Result<Self, AccountError> {
        let seed_digest = Hasher::hash_elements(&seed);

        // verify the seed digest satisfies all rules
        Self::validate_seed_digest(&seed_digest)?;

        // construct the ID from the first element of the seed hash
        let id = Self(seed_digest[0]);

        Ok(id)
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

    // SEED GENERATORS
    // --------------------------------------------------------------------------------------------

    /// Finds and returns a seed suitable for creating an account ID for the specified account type
    /// using the provided seed as a starting point.
    pub fn get_account_seed(
        init_seed: [u8; 32],
        account_type: AccountType,
        on_chain: bool,
    ) -> Result<Word, AccountError> {
        let init_seed: Vec<[u8; 8]> =
            init_seed.chunks(8).map(|chunk| chunk.try_into().unwrap()).collect();
        let mut current_seed: Word = [
            Felt::from(init_seed[0]),
            Felt::from(init_seed[1]),
            Felt::from(init_seed[2]),
            Felt::from(init_seed[3]),
        ];
        let mut current_digest = Hasher::hash_elements(&current_seed);

        // loop until we have a seed that satisfies the specified account type.
        loop {
            // check if the seed satisfies the specified account type
            if AccountId::validate_seed_digest(&current_digest).is_ok() {
                if let Ok(account_id) = AccountId::try_from(current_digest[0]) {
                    if account_id.account_type() == account_type
                        && account_id.is_on_chain() == on_chain
                    {
                        return Ok(current_seed);
                    };
                }
            }
            current_seed = current_digest.into();
            current_digest = Hasher::hash_elements(&current_seed);
        }
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

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
        let pow_trailing_zeros = elements[3].as_int().trailing_zeros();

        // check if there is there enough trailing zeros in the last element of the seed hash for
        // the account type.
        let sufficient_pow = match is_regular_account {
            true => pow_trailing_zeros >= Self::REGULAR_ACCOUNT_SEED_DIGEST_MIN_TRAILING_ZEROS,
            false => pow_trailing_zeros >= Self::FAUCET_SEED_DIGEST_MIN_TRAILING_ZEROS,
        };

        if !sufficient_pow {
            return Err(AccountError::seed_digest_too_few_trailing_zeros());
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
}

impl Deref for AccountId {
    type Target = Felt;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:02x}", self.as_int())
    }
}

impl Hash for AccountId {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.as_int().hash(state)
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn parse_felt(bytes: &[u8]) -> Result<Felt, AccountError> {
    Felt::try_from(bytes).map_err(|err| AccountError::AccountIdInvalidFieldElement(err.to_string()))
}
