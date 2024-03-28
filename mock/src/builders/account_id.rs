use alloc::string::{String, ToString};

use miden_objects::{
    accounts::{AccountId, AccountStorageType, AccountType},
    Digest, Word,
};
use rand::Rng;

use crate::{
    builders::{str_to_account_code, AccountBuilderError},
    mock::account::DEFAULT_ACCOUNT_CODE,
};

/// Builder for an `AccountId`, the builder can be configured and used multiple times.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct AccountIdBuilder<T> {
    account_type: AccountType,
    storage_type: AccountStorageType,
    code: String,
    storage_root: Digest,
    rng: T,
}

impl<T: Rng> AccountIdBuilder<T> {
    pub fn new(rng: T) -> Self {
        Self {
            account_type: AccountType::RegularAccountUpdatableCode,
            storage_type: AccountStorageType::OffChain,
            code: DEFAULT_ACCOUNT_CODE.to_string(),
            storage_root: Digest::default(),
            rng,
        }
    }

    pub fn account_type(&mut self, account_type: AccountType) -> &mut Self {
        self.account_type = account_type;
        self
    }

    pub fn storage_type(&mut self, storage_type: AccountStorageType) -> &mut Self {
        self.storage_type = storage_type;
        self
    }

    pub fn code<C: AsRef<str>>(&mut self, code: C) -> &mut Self {
        self.code = code.as_ref().to_string();
        self
    }

    pub fn storage_root(&mut self, storage_root: Digest) -> &mut Self {
        self.storage_root = storage_root;
        self
    }

    pub fn build(&mut self) -> Result<AccountId, AccountBuilderError> {
        let (seed, code_root) = accountid_build_details(
            &mut self.rng,
            &self.code,
            self.account_type,
            self.storage_type,
            self.storage_root,
        )?;

        AccountId::new(seed, code_root, self.storage_root)
            .map_err(AccountBuilderError::AccountError)
    }

    pub fn with_seed(&mut self, seed: Word) -> Result<AccountId, AccountBuilderError> {
        let code = str_to_account_code(&self.code).map_err(AccountBuilderError::AccountError)?;
        let code_root = code.root();

        let account_id = AccountId::new(seed, code_root, self.storage_root)
            .map_err(AccountBuilderError::AccountError)?;

        if account_id.account_type() != self.account_type {
            return Err(AccountBuilderError::SeedAndAccountTypeMismatch);
        }

        if account_id.storage_type() != self.storage_type {
            return Err(AccountBuilderError::SeedAndOnChainMismatch);
        }

        Ok(account_id)
    }
}

// UTILS
// ================================================================================================

/// Returns the account's seed and code root.
///
/// This compiles `code` and performs the proof-of-work to find a valid seed.
pub fn accountid_build_details<T: Rng>(
    rng: &mut T,
    code: &str,
    account_type: AccountType,
    storage_type: AccountStorageType,
    storage_root: Digest,
) -> Result<(Word, Digest), AccountBuilderError> {
    let init_seed: [u8; 32] = rng.gen();
    let code = str_to_account_code(code).map_err(AccountBuilderError::AccountError)?;
    let code_root = code.root();
    let seed =
        AccountId::get_account_seed(init_seed, account_type, storage_type, code_root, storage_root)
            .map_err(AccountBuilderError::AccountError)?;

    Ok((seed, code_root))
}
