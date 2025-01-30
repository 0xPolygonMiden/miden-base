use alloc::vec::Vec;

use vm_core::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::{DeserializationError, Digest};

use crate::{
    account::{delta::AccountUpdateDetails, AccountId},
    errors::BatchAccountUpdateError,
    transaction::{ProvenTransaction, TransactionId},
};

// ACCOUNT UPDATE
// ================================================================================================

/// Represents the changes made to an account resulting from executing a batch of transactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchAccountUpdate {
    /// ID of the updated account.
    account_id: AccountId,

    /// Commitment to the state of the account before this update is applied.
    ///
    /// Equal to `Digest::default()` for new accounts.
    initial_state_commitment: Digest,

    /// Commitment to the state of the account after this update is applied.
    final_state_commitment: Digest,

    /// IDs of all transactions that updated the account.
    transactions: Vec<TransactionId>,

    /// A set of changes which can be applied to the previous account state (i.e. `initial_state`)
    /// to get the new account state. For private accounts, this is set to
    /// [`AccountUpdateDetails::Private`].
    details: AccountUpdateDetails,
}

impl BatchAccountUpdate {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new [`BatchAccountUpdate`] instantiated from the provided parts.
    pub const fn new(
        account_id: AccountId,
        initial_state_commitment: Digest,
        final_state_commitment: Digest,
        transactions: Vec<TransactionId>,
        details: AccountUpdateDetails,
    ) -> Self {
        // TODO: Consider erroring if transactions.is_empty().
        Self {
            account_id,
            initial_state_commitment,
            final_state_commitment,
            transactions,
            details,
        }
    }

    /// Creates a [`BatchAccountUpdate`] by cloning the update and other details from the provided
    /// [`ProvenTransaction`].
    pub fn from_transaction(transaction: &ProvenTransaction) -> Self {
        Self::new(
            transaction.account_id(),
            transaction.account_update().init_state_hash(),
            transaction.account_update().final_state_hash(),
            vec![transaction.id()],
            transaction.account_update().details().clone(),
        )
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the ID of the updated account.
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Commitment to the state of the account before this update is applied.
    ///
    /// This is equal to [`Digest::default()`] for new accounts.
    pub fn initial_state_commitment(&self) -> Digest {
        self.initial_state_commitment
    }

    /// Commitment to the state of the account after this update is applied.
    pub fn final_state_commitment(&self) -> Digest {
        self.final_state_commitment
    }

    /// Returns a slice of [`TransactionId`]s that updated this account's state.
    pub fn transactions(&self) -> &[TransactionId] {
        &self.transactions
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

    // MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Merges the transaction's update into this account update.
    pub fn merge_proven_tx(
        &mut self,
        tx: &ProvenTransaction,
    ) -> Result<(), BatchAccountUpdateError> {
        if self.account_id != tx.account_id() {
            return Err(BatchAccountUpdateError::AccountUpdateIdMismatch {
                transaction: tx.id(),
                expected_account_id: self.account_id,
                actual_account_id: tx.account_id(),
            });
        }

        if self.final_state_commitment != tx.account_update().init_state_hash() {
            return Err(BatchAccountUpdateError::AccountUpdateInitialStateMismatch(tx.id()));
        }

        self.details = self.details.clone().merge(tx.account_update().details().clone()).map_err(
            |source_err| BatchAccountUpdateError::TransactionUpdateMergeError(tx.id(), source_err),
        )?;
        self.final_state_commitment = tx.account_update().final_state_hash();
        self.transactions.push(tx.id());

        Ok(())
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for BatchAccountUpdate {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.account_id.write_into(target);
        self.initial_state_commitment.write_into(target);
        self.final_state_commitment.write_into(target);
        self.transactions.write_into(target);
        self.details.write_into(target);
    }
}

impl Deserializable for BatchAccountUpdate {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        Ok(Self {
            account_id: AccountId::read_from(source)?,
            initial_state_commitment: Digest::read_from(source)?,
            final_state_commitment: Digest::read_from(source)?,
            transactions: <Vec<TransactionId>>::read_from(source)?,
            details: AccountUpdateDetails::read_from(source)?,
        })
    }
}
