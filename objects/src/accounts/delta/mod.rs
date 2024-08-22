use alloc::string::ToString;

use super::{
    Account, ByteReader, ByteWriter, Deserializable, DeserializationError, Felt, Serializable,
    Word, ZERO,
};
use crate::AccountDeltaError;

mod storage;
pub use storage::{AccountStorageDelta, StorageMapDelta};

mod vault;
pub use vault::{
    AccountVaultDelta, FungibleAssetDelta, NonFungibleAssetDelta, NonFungibleDeltaAction,
};

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
    nonce: Option<Felt>,
}

impl AccountDelta {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns new [AccountDelta] instantiated from the provided components.
    ///
    /// # Errors
    /// Returns an error if storage or vault were updated, but the nonce was either not updated
    /// or set to 0.
    pub fn new(
        storage: AccountStorageDelta,
        vault: AccountVaultDelta,
        nonce: Option<Felt>,
    ) -> Result<Self, AccountDeltaError> {
        // nonce must be updated if either account storage or vault were updated
        validate_nonce(nonce, &storage, &vault)?;

        Ok(Self { storage, vault, nonce })
    }

    /// Merge another [AccountDelta] into this one.
    pub fn merge(&mut self, other: Self) -> Result<(), AccountDeltaError> {
        match (&mut self.nonce, other.nonce) {
            (Some(old), Some(new)) if new.as_int() <= old.as_int() => {
                return Err(AccountDeltaError::InconsistentNonceUpdate(format!(
                    "New nonce {new} is not larger than the old nonce {old}"
                )))
            },
            // Incoming nonce takes precedence.
            (old, new) => *old = new.or(*old),
        };
        self.storage.merge(other.storage)?;
        self.vault.merge(other.vault)
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

    /// Returns the new nonce, if the nonce was changes.
    pub fn nonce(&self) -> Option<Felt> {
        self.nonce
    }

    /// Converts this storage delta into individual delta components.
    pub fn into_parts(self) -> (AccountStorageDelta, AccountVaultDelta, Option<Felt>) {
        (self.storage, self.vault, self.nonce)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AccountUpdateDetails {
    /// Account is private (no on-chain state change).
    Private,

    /// The whole state is needed for new accounts.
    New(Account),

    /// For existing accounts, only the delta is needed.
    Delta(AccountDelta),
}

impl AccountUpdateDetails {
    /// Returns `true` if the account update details are for private account.
    pub fn is_private(&self) -> bool {
        matches!(self, Self::Private)
    }

    /// Merges the `other` update into this one.
    ///
    /// This account update is assumed to come before the other.
    pub fn merge(self, other: AccountUpdateDetails) -> Result<Self, AccountDeltaError> {
        let merged_update = match (self, other) {
            (AccountUpdateDetails::Private, AccountUpdateDetails::Private) => {
                AccountUpdateDetails::Private
            },
            (AccountUpdateDetails::New(mut account), AccountUpdateDetails::Delta(delta)) => {
                account.apply_delta(&delta).map_err(|_| {
                    AccountDeltaError::IncompatibleAccountUpdates(
                        AccountUpdateDetails::New(account.clone()),
                        AccountUpdateDetails::Delta(delta),
                    )
                })?;

                AccountUpdateDetails::New(account)
            },
            (AccountUpdateDetails::Delta(mut delta), AccountUpdateDetails::Delta(new_delta)) => {
                delta.merge(new_delta)?;
                AccountUpdateDetails::Delta(delta)
            },
            (left, right) => {
                return Err(AccountDeltaError::IncompatibleAccountUpdates(left, right))
            },
        };

        Ok(merged_update)
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountDelta {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.storage.write_into(target);
        self.vault.write_into(target);
        self.nonce.write_into(target);
    }
}

impl Deserializable for AccountDelta {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let storage = AccountStorageDelta::read_from(source)?;
        let vault = AccountVaultDelta::read_from(source)?;
        let nonce = <Option<Felt>>::read_from(source)?;

        validate_nonce(nonce, &storage, &vault)
            .map_err(|err| DeserializationError::InvalidValue(err.to_string()))?;

        Ok(Self { storage, vault, nonce })
    }
}

impl Serializable for AccountUpdateDetails {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        match self {
            AccountUpdateDetails::Private => {
                0_u8.write_into(target);
            },
            AccountUpdateDetails::New(account) => {
                1_u8.write_into(target);
                account.write_into(target);
            },
            AccountUpdateDetails::Delta(delta) => {
                2_u8.write_into(target);
                delta.write_into(target);
            },
        }
    }
}

impl Deserializable for AccountUpdateDetails {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        match u8::read_from(source)? {
            0 => Ok(Self::Private),
            1 => Ok(Self::New(Account::read_from(source)?)),
            2 => Ok(Self::Delta(AccountDelta::read_from(source)?)),
            v => Err(DeserializationError::InvalidValue(format!(
                "Unknown variant {v} for AccountDetails"
            ))),
        }
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Checks if the nonce was updated correctly given the provided storage and vault deltas.
///
/// # Errors
/// Returns an error if storage or vault were updated, but the nonce was either not updated
/// or set to 0.
fn validate_nonce(
    nonce: Option<Felt>,
    storage: &AccountStorageDelta,
    vault: &AccountVaultDelta,
) -> Result<(), AccountDeltaError> {
    if !storage.is_empty() || !vault.is_empty() {
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
        let storage_delta = AccountStorageDelta::default();
        let vault_delta = AccountVaultDelta::default();

        assert!(AccountDelta::new(storage_delta.clone(), vault_delta.clone(), None).is_ok());
        assert!(AccountDelta::new(storage_delta.clone(), vault_delta.clone(), Some(ONE)).is_ok());

        // non-empty delta
        let storage_delta = AccountStorageDelta::from_iters([1], [], []);

        assert!(AccountDelta::new(storage_delta.clone(), vault_delta.clone(), None).is_err());
        assert!(AccountDelta::new(storage_delta.clone(), vault_delta.clone(), Some(ZERO)).is_err());
        assert!(AccountDelta::new(storage_delta.clone(), vault_delta.clone(), Some(ONE)).is_ok());
    }
}
