use rand::Rng;

use super::account_builder::AccountBuilderError;
use crate::{
    accounts::{AccountId, AccountStorageMode, AccountType},
    Digest, Word,
};

/// Builder for an `AccountId`, the builder can be configured and used multiple times.
#[derive(Debug, Clone)]
pub struct AccountIdBuilder<T> {
    account_type: AccountType,
    storage_mode: AccountStorageMode,
    code_commitment: Option<Digest>,
    storage_commitment: Digest,
    rng: T,
}

impl<T: Rng> AccountIdBuilder<T> {
    pub fn new(rng: T) -> Self {
        Self {
            account_type: AccountType::RegularAccountUpdatableCode,
            storage_mode: AccountStorageMode::Private,
            code_commitment: None,
            storage_commitment: Digest::default(),
            rng,
        }
    }

    pub fn account_type(&mut self, account_type: AccountType) -> &mut Self {
        self.account_type = account_type;
        self
    }

    pub fn storage_mode(&mut self, storage_mode: AccountStorageMode) -> &mut Self {
        self.storage_mode = storage_mode;
        self
    }

    pub fn code_commitment(&mut self, code_commitment: Digest) -> &mut Self {
        self.code_commitment = Some(code_commitment);
        self
    }

    pub fn storage_commitment(&mut self, storage_commitment: Digest) -> &mut Self {
        self.storage_commitment = storage_commitment;
        self
    }

    pub(crate) fn get_account_type(&self) -> AccountType {
        self.account_type
    }

    pub fn build(&mut self) -> Result<(AccountId, Word), AccountBuilderError> {
        let account_code = self.code_commitment.ok_or(AccountBuilderError::AccountCodeNotSet)?;

        let (seed, code_commitment) = account_id_build_details(
            &mut self.rng,
            account_code,
            self.account_type,
            self.storage_mode,
            self.storage_commitment,
        )?;

        let account_id = AccountId::new(seed, code_commitment, self.storage_commitment)
            .map_err(AccountBuilderError::AccountError)?;

        Ok((account_id, seed))
    }

    pub fn with_seed(&mut self, seed: Word) -> Result<AccountId, AccountBuilderError> {
        let code_commitment = self.code_commitment.ok_or(AccountBuilderError::AccountCodeNotSet)?;

        let account_id = AccountId::new(seed, code_commitment, self.storage_commitment)
            .map_err(AccountBuilderError::AccountError)?;

        if account_id.account_type() != self.account_type {
            return Err(AccountBuilderError::SeedAndAccountTypeMismatch);
        }

        if account_id.storage_mode() != self.storage_mode {
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
    code_commitment: Digest,
    account_type: AccountType,
    storage_mode: AccountStorageMode,
    storage_commitment: Digest,
) -> Result<(Word, Digest), AccountBuilderError> {
    let init_seed: [u8; 32] = rng.gen();
    let seed = AccountId::get_account_seed(
        init_seed,
        account_type,
        storage_mode,
        code_commitment,
        storage_commitment,
    )
    .map_err(AccountBuilderError::AccountError)?;

    Ok((seed, code_commitment))
}
