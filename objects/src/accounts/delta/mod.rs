use alloc::string::ToString;

use super::{
    AccountCode, ByteReader, ByteWriter, Deserializable, DeserializationError, Felt, Serializable,
    Word, ZERO,
};
use crate::{assets::Asset, AccountDeltaError};

mod storage;
pub use storage::AccountStorageDelta;

mod vault;
pub use vault::AccountVaultDelta;

// ACCOUNT DELTA
// ================================================================================================

/// [AccountDelta] stores the differences between two account states.
///
/// The differences are represented as follows:
/// - storage: an [AccountStorageDelta] that contains the changes to the account storage.
/// - vault: an [AccountVaultDelta] object that contains the changes to the account vault.
/// - nonce: if the nonce of the account has changed, the new nonce is stored here.
///
/// TODO: add ability to trace account code updates.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccountDelta {
    storage: AccountStorageDelta,
    vault: AccountVaultDelta,
    code: Option<AccountCode>,
    nonce: Option<Felt>,
}

impl AccountDelta {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns new [AccountDelta] instantiated from the provided components.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Storage or vault deltas are invalid.
    /// - Storage and vault deltas are empty, and the nonce was updated.
    /// - Storage or vault deltas are not empty, but nonce was not updated.
    pub fn new(
        storage: AccountStorageDelta,
        vault: AccountVaultDelta,
        code: Option<AccountCode>,
        nonce: Option<Felt>,
    ) -> Result<Self, AccountDeltaError> {
        // make sure storage and vault deltas are valid
        storage.validate()?;
        vault.validate()?;

        // nonce must be updated if and only if either account storage or vault were updated
        validate_nonce(nonce, &storage, code.as_ref(), &vault)?;

        Ok(Self { storage, vault, code, nonce })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns true if this account delta does not contain any updates.
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty() && self.vault.is_empty()
    }

    /// Returns storage updates for this account delta.
    pub fn storage(&self) -> &AccountStorageDelta {
        &self.storage
    }

    /// Returns vault updates for this account delta.
    pub fn vault(&self) -> &AccountVaultDelta {
        &self.vault
    }

    /// Returns initial code for this account delta (for new accounts).
    pub fn code(&self) -> Option<&AccountCode> {
        self.code.as_ref()
    }

    /// Returns the new nonce, if the nonce was changes.
    pub fn nonce(&self) -> Option<Felt> {
        self.nonce
    }

    /// Converts this storage delta into individual delta components.
    pub fn into_parts(self) -> (AccountStorageDelta, AccountVaultDelta, Option<Felt>) {
        (self.storage, self.vault, self.nonce)
    }
}

impl Serializable for AccountDelta {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.storage.write_into(target);
        self.vault.write_into(target);
        self.code.write_into(target);
        self.nonce.write_into(target);
    }
}

impl Deserializable for AccountDelta {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let storage = AccountStorageDelta::read_from(source)?;
        let vault = AccountVaultDelta::read_from(source)?;
        let code = <Option<AccountCode>>::read_from(source)?;
        let nonce = <Option<Felt>>::read_from(source)?;

        validate_nonce(nonce, &storage, code.as_ref(), &vault)
            .map_err(|err| DeserializationError::InvalidValue(err.to_string()))?;

        Ok(Self { storage, vault, code, nonce })
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Checks if the nonce was updated correctly given the provided storage and vault deltas.
///
/// # Errors
/// Returns an error if:
/// - Storage or vault were updated, but the nonce was either not updated or set to 0.
/// - Storage and vault were not updated, but the nonce was updated.
fn validate_nonce(
    nonce: Option<Felt>,
    storage: &AccountStorageDelta,
    code: Option<&AccountCode>,
    vault: &AccountVaultDelta,
) -> Result<(), AccountDeltaError> {
    if !storage.is_empty() || code.is_some() || !vault.is_empty() {
        match nonce {
            Some(nonce) => {
                if nonce == ZERO {
                    return Err(AccountDeltaError::InconsistentNonceUpdate(
                        "zero nonce for a non-empty account delta".to_string(),
                    ));
                }
            },
            None => {
                return Err(AccountDeltaError::InconsistentNonceUpdate(
                    "nonce not updated for non-empty account delta".to_string(),
                ))
            },
        }
    } else if nonce.is_some() {
        return Err(AccountDeltaError::InconsistentNonceUpdate(
            "nonce updated for empty delta".to_string(),
        ));
    }

    Ok(())
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::{AccountDelta, AccountStorageDelta, AccountVaultDelta};
    use crate::{ONE, ZERO};

    #[test]
    fn account_delta_nonce_validation() {
        // empty delta
        let storage_delta = AccountStorageDelta {
            cleared_items: vec![],
            updated_items: vec![],
        };

        let vault_delta = AccountVaultDelta {
            added_assets: vec![],
            removed_assets: vec![],
        };

        assert!(AccountDelta::new(storage_delta.clone(), vault_delta.clone(), None, None).is_ok());
        assert!(
            AccountDelta::new(storage_delta.clone(), vault_delta.clone(), None, Some(ONE)).is_err()
        );

        // non-empty delta
        let storage_delta = AccountStorageDelta {
            cleared_items: vec![1],
            updated_items: vec![],
        };

        assert!(AccountDelta::new(storage_delta.clone(), vault_delta.clone(), None, None).is_err());
        assert!(AccountDelta::new(storage_delta.clone(), vault_delta.clone(), None, Some(ZERO))
            .is_err());
        assert!(
            AccountDelta::new(storage_delta.clone(), vault_delta.clone(), None, Some(ONE)).is_ok()
        );
    }
}
