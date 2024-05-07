use alloc::string::{String, ToString};

use miden_objects::{
    accounts::{
        account_id::{AccountConfig, AccountPoW},
        AccountId, AccountStorageType, AccountType,
    },
    AccountError, Digest, Word,
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
    pow: Option<AccountPoW>,
    code: String,
    storage_root: Digest,
    rng: T,
}

impl<T: Rng> AccountIdBuilder<T> {
    pub fn new(rng: T) -> Self {
        Self {
            account_type: AccountType::RegularAccountUpdatableCode,
            storage_type: AccountStorageType::OffChain,
            pow: None,
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

    pub fn pow(&mut self, pow: u8) -> Result<&mut Self, AccountBuilderError> {
        let pow = AccountPoW::new(pow).map_err(AccountBuilderError::AccountError)?;
        self.pow = Some(pow);
        Ok(self)
    }

    pub fn build(&mut self) -> Result<AccountId, AccountBuilderError> {
        let config = self.make_config().map_err(AccountBuilderError::AccountError)?;
        let (seed, code_root) =
            accountid_build_details(&mut self.rng, &self.code, config, self.storage_root)?;

        AccountId::new(seed, config, code_root, self.storage_root)
            .map_err(AccountBuilderError::AccountError)
    }

    pub fn with_seed(&mut self, seed: Word) -> Result<AccountId, AccountBuilderError> {
        let config = self.make_config().map_err(AccountBuilderError::AccountError)?;
        let code = str_to_account_code(&self.code).map_err(AccountBuilderError::AccountError)?;
        let code_root = code.root();

        let account_id = AccountId::new(seed, config, code_root, self.storage_root)
            .map_err(AccountBuilderError::AccountError)?;

        if account_id.account_type() != self.account_type {
            return Err(AccountBuilderError::SeedAndAccountTypeMismatch);
        }

        if account_id.storage_type() != self.storage_type {
            return Err(AccountBuilderError::SeedAndOnChainMismatch);
        }

        Ok(account_id)
    }

    fn make_config(&self) -> Result<AccountConfig, AccountError> {
        if let Some(pow) = self.pow {
            AccountConfig::new_with_pow(self.account_type, self.storage_type, pow)
        } else {
            Ok(AccountConfig::new(self.account_type, self.storage_type))
        }
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
    config: AccountConfig,
    storage_root: Digest,
) -> Result<(Word, Digest), AccountBuilderError> {
    let init_seed: [u8; 32] = rng.gen();
    let code = str_to_account_code(code).map_err(AccountBuilderError::AccountError)?;
    let code_root = code.root();
    let seed = AccountId::get_account_seed(init_seed, config, code_root, storage_root)
        .map_err(AccountBuilderError::AccountError)?;

    Ok((seed, code_root))
}
