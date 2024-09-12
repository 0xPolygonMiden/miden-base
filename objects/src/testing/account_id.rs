use assembly::Assembler;
use rand::Rng;

use super::{account::AccountBuilderError, account_code::DEFAULT_ACCOUNT_CODE};
use crate::{
    accounts::{AccountCode, AccountId, AccountStorageMode, AccountType},
    Digest, Word,
};

/// Builder for an `AccountId`, the builder can be configured and used multiple times.
#[derive(Debug, Clone)]
pub struct AccountIdBuilder<T> {
    account_type: AccountType,
    storage_mode: AccountStorageMode,
    code: Option<AccountCode>,
    storage_commitment: Digest,
    rng: T,
}

impl<T: Rng> AccountIdBuilder<T> {
    pub fn new(rng: T) -> Self {
        Self {
            account_type: AccountType::RegularAccountUpdatableCode,
            storage_mode: AccountStorageMode::Private,
            code: None,
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

    pub fn code(&mut self, code: AccountCode) -> &mut Self {
        self.code = Some(code);
        self
    }

    /// Compiles [DEFAULT_ACCOUNT_CODE] into [AccountCode] and sets it.
    pub fn default_code(mut self, assembler: Assembler) -> Self {
        self.code = Some(
            AccountCode::compile(DEFAULT_ACCOUNT_CODE, assembler)
                .expect("Default account code should compile."),
        );
        self
    }

    pub fn storage_commitment(&mut self, storage_commitment: Digest) -> &mut Self {
        self.storage_commitment = storage_commitment;
        self
    }

    pub fn build(&mut self) -> Result<(AccountId, Word), AccountBuilderError> {
        let account_code = self.code.clone().ok_or(AccountBuilderError::AccountCodeNotSet)?;

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
        let account_code = self.code.clone().ok_or(AccountBuilderError::AccountCodeNotSet)?;
        let code_commitment = account_code.commitment();

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
    code: AccountCode,
    account_type: AccountType,
    storage_mode: AccountStorageMode,
    storage_commitment: Digest,
) -> Result<(Word, Digest), AccountBuilderError> {
    let init_seed: [u8; 32] = rng.gen();
    let code_commitment = code.commitment();
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
