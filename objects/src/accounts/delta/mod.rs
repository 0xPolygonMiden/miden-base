use alloc::string::ToString;

use vm_core::WORD_SIZE;

use super::{
    Account, ByteReader, ByteWriter, Deserializable, DeserializationError, Felt, Serializable,
    Word, ZERO,
};
use crate::{AccountDeltaError, ACCOUNT_DELTA_MAX_SIZE};

mod storage;
pub use storage::{AccountStorageDelta, StorageMapDelta};

mod vault;
pub use vault::{
    AccountVaultDelta, FungibleAssetDelta, NonFungibleAssetDelta, NonFungibleDeltaAction,
};

/// The serialized size of a [`Felt`] which is serialized as a [`u64`].
const FELT_SERIALIZED_SIZE: usize = std::mem::size_of::<u64>();
/// The serialized size of a [`Word`].
const WORD_SERIALIZED_SIZE: usize = FELT_SERIALIZED_SIZE * WORD_SIZE;
/// The serialized size of a [`u8`].
const U8_SERIALIZED_SIZE: usize = std::mem::size_of::<u8>();
/// The serialized size of a [`u64`].
const U64_SERIALIZED_SIZE: usize = std::mem::size_of::<u64>();
/// The serialized size of a [`Digest`].
/// This is a copy of `miden_crypto::hash::rescue::DIGEST_BYTES`.
const DIGEST_SERIALIZED_SIZE: usize = 32;

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
    ///
    /// - Returns an error if storage or vault were updated, but the nonce was either not updated or
    ///   set to 0.
    /// - Returns an error if the serialized size of the delta exceeds the maximum allowed size.
    pub fn new(
        storage: AccountStorageDelta,
        vault: AccountVaultDelta,
        nonce: Option<Felt>,
    ) -> Result<Self, AccountDeltaError> {
        // nonce must be updated if either account storage or vault were updated
        validate_nonce(nonce, &storage, &vault)?;

        let account_delta = Self { storage, vault, nonce };

        account_delta.validate_max_size()?;

        Ok(account_delta)
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

    // VALIDATION
    // --------------------------------------------------------------------------------------------

    /// Validates that the delta's size does not exceed the maximum allowed size.
    fn validate_max_size(&self) -> Result<(), AccountDeltaError> {
        if self.get_size_hint() > ACCOUNT_DELTA_MAX_SIZE as usize {
            Err(AccountDeltaError::SizeLimitExceeded)
        } else {
            Ok(())
        }
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

    fn get_size_hint(&self) -> usize {
        self.storage.get_size_hint()
            + self.vault.get_size_hint()
            // An option's tag is serialized as a bool which is always 1 byte.
            + 1
            + self.nonce.map(|_nonce| FELT_SERIALIZED_SIZE).unwrap_or(0)
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

/// Returns the length of `value` in vint64 encoding.
/// Copy from winter-utils.
pub(super) fn usize_encoded_len(value: usize) -> usize {
    // winter-utils casts the usize to a u64 during encoding.
    let value = value as u64;
    let zeros = value.leading_zeros() as usize;
    let len = zeros.saturating_sub(1) / 7;
    9 - core::cmp::min(len, 8)
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeMap;

    use vm_core::{utils::Serializable, Felt};
    use vm_processor::Digest;

    use super::{AccountDelta, AccountStorageDelta, AccountVaultDelta};
    use crate::{
        accounts::{delta::WORD_SERIALIZED_SIZE, AccountId, AccountType, StorageMapDelta},
        assets::{Asset, FungibleAsset, NonFungibleAsset, NonFungibleAssetDetails},
        AccountDeltaError, ACCOUNT_DELTA_MAX_SIZE, ONE, ZERO,
    };

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

    #[test]
    fn account_delta_size_hint() {
        let storage_delta = AccountStorageDelta::default();
        let vault_delta = AccountVaultDelta::default();
        assert_eq!(storage_delta.to_bytes().len(), storage_delta.get_size_hint());
        assert_eq!(vault_delta.to_bytes().len(), vault_delta.get_size_hint());

        let account_delta = AccountDelta::new(storage_delta, vault_delta, None).unwrap();
        assert_eq!(account_delta.to_bytes().len(), account_delta.get_size_hint());

        let storage_delta = AccountStorageDelta::from_iters(
            [1],
            [(2, [ONE, ONE, ONE, ONE]), (3, [ONE, ONE, ZERO, ONE])],
            [(
                4,
                StorageMapDelta::from_iters(
                    [[ONE, ONE, ONE, ZERO], [ZERO, ONE, ONE, ONE]],
                    [([ONE, ONE, ONE, ONE], [ONE, ONE, ONE, ONE])],
                ),
            )],
        );

        let non_fungible: Asset = NonFungibleAsset::new(
            &NonFungibleAssetDetails::new(
                AccountId::new_dummy([10; 32], AccountType::NonFungibleFaucet),
                vec![6],
            )
            .unwrap(),
        )
        .unwrap()
        .into();
        let fungible_2: Asset =
            FungibleAsset::new(AccountId::new_dummy([10; 32], AccountType::FungibleFaucet), 10)
                .unwrap()
                .into();
        let vault_delta = AccountVaultDelta::from_iters([non_fungible], [fungible_2]);

        assert_eq!(storage_delta.to_bytes().len(), storage_delta.get_size_hint());
        assert_eq!(vault_delta.to_bytes().len(), vault_delta.get_size_hint());

        let account_delta = AccountDelta::new(storage_delta, vault_delta, Some(ONE)).unwrap();
        assert_eq!(account_delta.to_bytes().len(), account_delta.get_size_hint());
    }

    #[test]
    fn account_delta_size_limit() {
        // A small delta does not exceed the limit.
        let storage_delta = AccountStorageDelta::from_iters(
            [1, 2, 3, 4],
            [(2, [ONE, ONE, ONE, ONE]), (3, [ONE, ONE, ZERO, ONE])],
            [],
        );
        AccountDelta::new(storage_delta, AccountVaultDelta::default(), Some(ONE)).unwrap();

        let mut map = BTreeMap::new();
        // The number of entries in the map required to exceed the limit.
        let required_entries = ACCOUNT_DELTA_MAX_SIZE / (2 * WORD_SERIALIZED_SIZE as u16);
        for _ in 0..required_entries {
            map.insert(
                Digest::new([
                    Felt::new(rand::random()),
                    Felt::new(rand::random()),
                    Felt::new(rand::random()),
                    Felt::new(rand::random()),
                ]),
                [
                    Felt::new(rand::random()),
                    Felt::new(rand::random()),
                    Felt::new(rand::random()),
                    Felt::new(rand::random()),
                ],
            );
        }
        let storage_delta = StorageMapDelta::new(map);

        // A delta that exceeds the limit returns an error.
        let storage_delta = AccountStorageDelta::from_iters([], [], [(4, storage_delta)]);
        let err =
            AccountDelta::new(storage_delta, AccountVaultDelta::default(), Some(ONE)).unwrap_err();
        assert!(matches!(err, AccountDeltaError::SizeLimitExceeded));
    }
}
