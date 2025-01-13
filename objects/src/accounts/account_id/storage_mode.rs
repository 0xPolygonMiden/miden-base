use alloc::string::String;
use core::{fmt, str::FromStr};

use crate::errors::AccountIdError;

// ACCOUNT STORAGE MODE
// ================================================================================================

pub(super) const PUBLIC: u8 = 0b00;
pub(super) const PRIVATE: u8 = 0b10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
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
    type Error = AccountIdError;

    fn try_from(value: &str) -> Result<Self, AccountIdError> {
        match value.to_lowercase().as_str() {
            "public" => Ok(AccountStorageMode::Public),
            "private" => Ok(AccountStorageMode::Private),
            _ => Err(AccountIdError::UnknownAccountStorageMode(value.into())),
        }
    }
}

impl TryFrom<String> for AccountStorageMode {
    type Error = AccountIdError;

    fn try_from(value: String) -> Result<Self, AccountIdError> {
        AccountStorageMode::from_str(&value)
    }
}

impl FromStr for AccountStorageMode {
    type Err = AccountIdError;

    fn from_str(input: &str) -> Result<AccountStorageMode, AccountIdError> {
        AccountStorageMode::try_from(input)
    }
}

#[cfg(any(feature = "testing", test))]
impl rand::distributions::Distribution<AccountStorageMode> for rand::distributions::Standard {
    /// Samples a uniformly random [`AccountStorageMode`] from the given `rng`.
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> AccountStorageMode {
        match rng.gen_range(0..2) {
            0 => AccountStorageMode::Public,
            1 => AccountStorageMode::Private,
            _ => unreachable!("gen_range should not produce higher values"),
        }
    }
}
