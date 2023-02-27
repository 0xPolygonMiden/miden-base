use super::{AccountError, Felt, Hasher, StarkField, ToString, Word, ZERO};
use core::{fmt, ops::Deref};

// ACCOUNT ID
// ================================================================================================

/// Unique identifier of an account.
///
/// Account ID consists of 3 field elements (24 bytes). These field elements uniquely identify a
/// single account and also specify the type of the underlying account. Specifically:
/// - If the least significant 32 bits of the 3rd element are all ZEROs, the account is a
///   fungible asset faucet (i.e., it can issue fungible assets).
/// - If the least significant 32 bits of the 3rd element are set to 2^31 (i.e ONE followed by 31
///   ZEROs), the account is a non-fungible asset faucet (i.e., it can issue non-fungible assets).
/// - Otherwise, the account is a regular account.
///
/// Additionally, account IDs have the following properties
/// - For fungible asset faucets account IDs are guaranteed to start with ONE.
/// - For regular accounts, the last 3 bytes of the ID are guaranteed to be all ZEROs.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AccountId([Felt; 3]);

impl AccountId {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------
    pub const FUNGIBLE_FAUCET_TAG: u32 = 0;
    pub const NON_FUNGIBLE_FAUCET_TAG: u32 = 1 << 31;

    /// Specifies a minimum number of trailing zeros for a valid account ID. Thus, all valid
    /// account IDs have the last 3 bytes set to zeros.
    pub const MIN_TRAILING_ZEROS: u32 = 24;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new account ID derived from the specified seed.
    ///
    /// The account ID is computed by hashing the seed and using 3 elements of the result to form
    /// the ID. Specifically we are take elements 0, 1, and 3, omitting element 2. We omit element
    /// 2 because unlike elements 0 and 3, it has no special structure which we need to carry over
    /// into the derived account ID. Element 1 could have been omitted just as well.
    ///
    /// # Errors
    /// Returns an error if the resulting account ID does not comply with account ID rules.
    pub fn new(seed: Word) -> Result<Self, AccountError> {
        // hash the seed and build the ID from the 1st, 2nd, and 4th elements of the result
        let hash = Hasher::hash_elements(&seed);
        let id = Self([hash[0], hash[1], hash[3]]);

        // verify that the ID satisfies all rules
        id.validate()?;

        Ok(id)
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns true if an account with this ID can issue fungible assets.
    pub fn is_fungible_faucet(&self) -> bool {
        self.tag() == Self::FUNGIBLE_FAUCET_TAG
    }

    /// Returns true if an account with this ID can issue non-fungible assets.
    pub fn is_non_fungible_faucet(&self) -> bool {
        self.tag() == Self::NON_FUNGIBLE_FAUCET_TAG
    }

    /// Returns true if an account with this ID can issue assets.
    pub fn is_faucet(&self) -> bool {
        self.is_fungible_faucet() || self.is_non_fungible_faucet()
    }

    /// Returns a slice of field elements defining this account ID.
    pub fn as_elements(&self) -> &[Felt] {
        &self.0
    }

    // SEED GENERATORS
    // --------------------------------------------------------------------------------------------

    /// Finds and returns a seed suitable for creating regular account IDs using the provided seed
    /// as a starting point.
    pub fn get_account_seed(_init_seed: [u8; 32]) -> Word {
        todo!()
    }

    /// Finds and returns a seed suitable for creating account IDs for fungible faucets using the
    /// provided seed as a starting point.
    pub fn get_fungible_faucet_seed(_init_seed: [u8; 32]) -> Word {
        todo!()
    }

    /// Finds and returns a seed suitable for creating account IDs for non-fungible faucets using
    /// the provided seed as a starting point.
    pub fn get_non_fungible_faucet_seed(_init_seed: [u8; 32]) -> Word {
        todo!()
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    /// Returns the first bit of this account ID.
    fn first_bit(&self) -> u8 {
        (self.0[0].as_int() >> 63) as u8
    }

    /// Returns the last 32 bits of this account ID.
    fn tag(&self) -> u32 {
        self.0[2].as_int() as u32
    }

    /// Returns an error if:
    /// - This account ID is for a fungible asset but the first bit of the ID is not ONE.
    /// - There are fewer than 24 trailing ZEROs in this account ID.
    fn validate(&self) -> Result<(), AccountError> {
        if self.is_fungible_faucet() {
            // IDs for fungible faucets must start with ONE
            if self.first_bit() != 1 {
                return Err(AccountError::fungible_faucet_id_invalid_first_bit());
            }
        } else if self.tag().trailing_zeros() < Self::MIN_TRAILING_ZEROS {
            // all account IDs must end with at least 24 ZEROs
            return Err(AccountError::account_id_too_few_trailing_zeros());
        }

        Ok(())
    }
}

impl Deref for AccountId {
    type Target = [Felt; 3];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<AccountId> for [Felt; 3] {
    fn from(id: AccountId) -> Self {
        id.0
    }
}

impl From<AccountId> for Word {
    fn from(id: AccountId) -> Self {
        [id.0[0], id.0[1], id.0[2], ZERO]
    }
}

impl From<AccountId> for [u8; 24] {
    fn from(id: AccountId) -> Self {
        let mut result = [0_u8; 24];
        result[..8].copy_from_slice(&id.0[0].as_int().to_le_bytes());
        result[8..16].copy_from_slice(&id.0[1].as_int().to_le_bytes());
        result[16..].copy_from_slice(&id.0[2].as_int().to_le_bytes());
        result
    }
}

/// This conversion is possible because the 3 least significant bytes of an account ID are always
/// set to zeros.
impl From<AccountId> for [u8; 21] {
    fn from(id: AccountId) -> Self {
        let mut result = [0_u8; 21];
        result[..8].copy_from_slice(&id.0[0].as_int().to_le_bytes());
        result[8..16].copy_from_slice(&id.0[1].as_int().to_le_bytes());
        result[16..].copy_from_slice(&id.0[2].as_int().to_le_bytes()[..5]);
        result
    }
}

impl TryFrom<[Felt; 3]> for AccountId {
    type Error = AccountError;

    fn try_from(value: [Felt; 3]) -> Result<Self, Self::Error> {
        let id = Self(value);
        id.validate()?;
        Ok(id)
    }
}

impl TryFrom<[u8; 24]> for AccountId {
    type Error = AccountError;

    fn try_from(value: [u8; 24]) -> Result<Self, Self::Error> {
        let elements =
            [parse_felt(&value[..8])?, parse_felt(&value[8..16])?, parse_felt(&value[16..])?];
        Self::try_from(elements)
    }
}

impl TryFrom<[u8; 21]> for AccountId {
    type Error = AccountError;

    fn try_from(value: [u8; 21]) -> Result<Self, Self::Error> {
        let mut bytes = [0_u8; 24];
        bytes[..21].copy_from_slice(&value);
        Self::try_from(bytes)
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn parse_felt(bytes: &[u8]) -> Result<Felt, AccountError> {
    Felt::try_from(bytes).map_err(|err| AccountError::AccountIdInvalidFieldElement(err.to_string()))
}
