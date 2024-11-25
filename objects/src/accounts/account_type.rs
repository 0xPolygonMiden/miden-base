// ACCOUNT TYPES
// ================================================================================================

use crate::accounts::ACCOUNT_TYPE_MASK;

pub const FUNGIBLE_FAUCET: u64 = 0b10;
pub const NON_FUNGIBLE_FAUCET: u64 = 0b11;
pub const REGULAR_ACCOUNT_IMMUTABLE_CODE: u64 = 0b00;
pub const REGULAR_ACCOUNT_UPDATABLE_CODE: u64 = 0b01;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u64)]
pub enum AccountType2 {
    FungibleFaucet = FUNGIBLE_FAUCET,
    NonFungibleFaucet = NON_FUNGIBLE_FAUCET,
    RegularAccountImmutableCode = REGULAR_ACCOUNT_IMMUTABLE_CODE,
    RegularAccountUpdatableCode = REGULAR_ACCOUNT_UPDATABLE_CODE,
}

impl AccountType2 {
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
pub const fn account_type_from_u64(value: u64) -> AccountType2 {
    debug_assert!(
        ACCOUNT_TYPE_MASK.count_ones() == 2,
        "This method assumes there are only 2bits in the mask"
    );

    let bits = value & ACCOUNT_TYPE_MASK;
    match bits {
        REGULAR_ACCOUNT_UPDATABLE_CODE => AccountType2::RegularAccountUpdatableCode,
        REGULAR_ACCOUNT_IMMUTABLE_CODE => AccountType2::RegularAccountImmutableCode,
        FUNGIBLE_FAUCET => AccountType2::FungibleFaucet,
        NON_FUNGIBLE_FAUCET => AccountType2::NonFungibleFaucet,
        _ => {
            // account_type mask contains 2 bits and we exhaustively match all 4 possible options
            unreachable!()
        },
    }
}

/// Returns the [AccountType2] given an integer representation of `account_id`.
impl From<u128> for AccountType2 {
    fn from(value: u128) -> Self {
        let val = (value >> 64) as u64;
        std::println!("val {:b}", val);
        account_type_from_u64(val)
    }
}
