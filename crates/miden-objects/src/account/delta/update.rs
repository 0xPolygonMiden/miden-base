use alloc::vec::Vec;

use vm_core::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::{DeserializationError, Digest};

use crate::{
    account::{delta::AccountUpdateDetails, AccountId},
    errors::AccountUpdateError,
    transaction::{ProvenTransaction, TransactionId},
    ProvenTransactionError,
};

// ACCOUNT UPDATE
// ================================================================================================

/// Describes the changes made to an account state resulting from executing a single transaction, a
/// batch of transactions or multiple transaction batches in a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountUpdate {
    /// ID of the updated account.
    account_id: AccountId,

    /// Commitment to the state of the account before this update is applied.
    ///
    /// Equal to `Digest::default()` for new accounts.
    initial_state_commitment: Digest,

    /// Commitment to the state of the account after this update is applied.
    final_state_commitment: Digest,

    /// A set of changes which can be applied to the previous account state (i.e. `initial_state`)
    /// to get the new account state. For private accounts, this is set to
    /// [`AccountUpdateDetails::Private`].
    details: AccountUpdateDetails,

    /// IDs of all transactions that updated the account.
    transactions: Vec<TransactionId>,
}

impl AccountUpdate {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// The maximum allowed size of an account update. Set to 32 KiB.
    pub const ACCOUNT_UPDATE_MAX_SIZE: u16 = 2u16.pow(15);

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new [`AccountUpdate`] instantiated from the provided parts.
    pub const fn new(
        account_id: AccountId,
        initial_state_commitment: Digest,
        final_state_commitment: Digest,
        details: AccountUpdateDetails,
        transactions: Vec<TransactionId>,
    ) -> Self {
        Self {
            account_id,
            initial_state_commitment,
            final_state_commitment,
            details,
            transactions,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the ID of the updated account.
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Returns the commitment to the account's initial state.
    pub fn initial_state_commitment(&self) -> Digest {
        self.initial_state_commitment
    }

    /// Returns the commitment to the account's final state.
    pub fn final_state_commitment(&self) -> Digest {
        self.final_state_commitment
    }

    /// Returns the contained [`AccountUpdateDetails`].
    ///
    /// This update can be used to build the new account state from the previous account state.
    pub fn details(&self) -> &AccountUpdateDetails {
        &self.details
    }

    /// Returns `true` if the account update details are for a private account.
    pub fn is_private(&self) -> bool {
        self.details.is_private()
    }

    /// Validates the following properties of the account update:
    ///
    /// - The size of the serialized account update does not exceed [`ACCOUNT_UPDATE_MAX_SIZE`].
    pub(crate) fn validate(&self) -> Result<(), ProvenTransactionError> {
        let account_update_size = self.details().get_size_hint();
        if account_update_size > Self::ACCOUNT_UPDATE_MAX_SIZE as usize {
            Err(ProvenTransactionError::AccountUpdateSizeLimitExceeded {
                account_id: self.account_id(),
                update_size: account_update_size,
            })
        } else {
            Ok(())
        }
    }

    // MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Merges the transaction's update into this account update.
    fn merge_proven_tx(&mut self, tx: &ProvenTransaction) -> Result<(), AccountUpdateError> {
        if self.account_id != tx.account_id() {
            return Err(AccountUpdateError::AccountUpdateIdMismatch {
                transaction: tx.id(),
                expected_account_id: self.account_id,
                actual_account_id: tx.account_id(),
            });
        }

        if self.final_state_commitment != tx.account_update().init_state_hash() {
            return Err(AccountUpdateError::AccountUpdateInitialStateMismatch(tx.id()));
        }

        self.details = self.details.clone().merge(tx.account_update().details().clone()).map_err(
            |source_err| AccountUpdateError::TransactionUpdateMergeError(tx.id(), source_err),
        )?;
        self.final_state_commitment = tx.account_update().final_state_hash();
        self.transactions.push(tx.id());

        Ok(())
    }
}

// CONVERSIONS TO ACCOUNT UPDATE
// ================================================================================================

impl From<&ProvenTransaction> for AccountUpdate {
    fn from(tx: &ProvenTransaction) -> Self {
        Self {
            account_id: tx.account_id(),
            initial_state_commitment: tx.account_update().init_state_hash(),
            final_state_commitment: tx.account_update().final_state_hash(),
            transactions: vec![tx.id()],
            details: tx.account_update().details().clone(),
        }
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountUpdate {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.account_id.write_into(target);
        self.initial_state_commitment.write_into(target);
        self.final_state_commitment.write_into(target);
        self.details.write_into(target);
        self.transactions.write_into(target);
    }
}

impl Deserializable for AccountUpdate {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        Ok(Self {
            account_id: AccountId::read_from(source)?,
            initial_state_commitment: Digest::read_from(source)?,
            final_state_commitment: Digest::read_from(source)?,
            details: AccountUpdateDetails::read_from(source)?,
            transactions: <Vec<TransactionId>>::read_from(source)?,
        })
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeMap, vec::Vec};

    use winter_rand_utils::rand_array;

    use crate::{
        account::{
            delta::{AccountUpdate, AccountUpdateDetails},
            AccountDelta, AccountId, AccountStorageDelta, AccountVaultDelta, StorageMapDelta,
        },
        testing::account_id::ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        utils::Serializable,
        Digest, ProvenTransactionError, ACCOUNT_UPDATE_MAX_SIZE, EMPTY_WORD, ONE, ZERO,
    };

    #[test]
    fn account_update_size_limit_not_exceeded() {
        // A small delta does not exceed the limit.
        let storage_delta = AccountStorageDelta::from_iters(
            [1, 2, 3, 4],
            [(2, [ONE, ONE, ONE, ONE]), (3, [ONE, ONE, ZERO, ONE])],
            [],
        );
        let delta =
            AccountDelta::new(storage_delta, AccountVaultDelta::default(), Some(ONE)).unwrap();
        let details = AccountUpdateDetails::Delta(delta);
        AccountUpdate::new(
            AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap(),
            Digest::new(EMPTY_WORD),
            Digest::new(EMPTY_WORD),
            details,
            Vec::new(),
        )
        .validate()
        .unwrap();
    }

    #[test]
    fn account_update_size_limit_exceeded() {
        let mut map = BTreeMap::new();
        // The number of entries in the map required to exceed the limit.
        // We divide by each entry's size which consists of a key (digest) and a value (word), both
        // 32 bytes in size.
        let required_entries = ACCOUNT_UPDATE_MAX_SIZE / (2 * 32);
        for _ in 0..required_entries {
            map.insert(Digest::new(rand_array()), rand_array());
        }
        let storage_delta = StorageMapDelta::new(map);

        // A delta that exceeds the limit returns an error.
        let storage_delta = AccountStorageDelta::from_iters([], [], [(4, storage_delta)]);
        let delta =
            AccountDelta::new(storage_delta, AccountVaultDelta::default(), Some(ONE)).unwrap();
        let details = AccountUpdateDetails::Delta(delta);
        let details_size = details.get_size_hint();

        let err = AccountUpdate::new(
            AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap(),
            Digest::new(EMPTY_WORD),
            Digest::new(EMPTY_WORD),
            details,
            Vec::new(),
        )
        .validate()
        .unwrap_err();

        assert!(
            matches!(err, ProvenTransactionError::AccountUpdateSizeLimitExceeded { update_size, .. } if update_size == details_size)
        );
    }
}
