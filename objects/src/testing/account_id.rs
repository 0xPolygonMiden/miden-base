use alloc::string::{String, ToString};

use assembly::Assembler;
use rand::Rng;

use super::{
    account::AccountBuilderError, account_code::DEFAULT_ACCOUNT_CODE, str_to_account_code,
};
use crate::{
    accounts::{AccountId, AccountStorageType, AccountType},
    Digest, Word,
};

/// Builder for an `AccountId`, the builder can be configured and used multiple times.
#[derive(Debug, Clone)]
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

    pub fn build(
        &mut self,
        assembler: &Assembler,
    ) -> Result<(AccountId, Word), AccountBuilderError> {
        let (seed, code_commitment) = account_id_build_details(
            &mut self.rng,
            &self.code,
            self.account_type,
            self.storage_type,
            self.storage_root,
            assembler,
        )?;

        let account_id = AccountId::new(seed, code_commitment, self.storage_root)
            .map_err(AccountBuilderError::AccountError)?;

        Ok((account_id, seed))
    }

    pub fn with_seed(
        &mut self,
        seed: Word,
        assembler: &Assembler,
    ) -> Result<AccountId, AccountBuilderError> {
        let code = str_to_account_code(&self.code, assembler)
            .map_err(AccountBuilderError::AccountError)?;
        let code_commitment = code.commitment();

        let account_id = AccountId::new(seed, code_commitment, self.storage_root)
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

/// Returns the account's seed and code commitment.
///
/// This compiles `code` and performs the proof-of-work to find a valid seed.
pub fn account_id_build_details<T: Rng>(
    rng: &mut T,
    code: &str,
    account_type: AccountType,
    storage_type: AccountStorageType,
    storage_root: Digest,
    assembler: &Assembler,
) -> Result<(Word, Digest), AccountBuilderError> {
    let init_seed: [u8; 32] = rng.gen();
    let code = str_to_account_code(code, assembler).map_err(AccountBuilderError::AccountError)?;
    let code_commitment = code.commitment();
    let seed = AccountId::get_account_seed(
        init_seed,
        account_type,
        storage_type,
        code_commitment,
        storage_root,
    )
    .map_err(AccountBuilderError::AccountError)?;

    Ok((seed, code_commitment))
}
