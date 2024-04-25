use crate::{
    accounts::{delta::AccountUpdateDetails, AccountId},
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest,
};

/// Describes the changes made to the account state resulting from a transaction execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxAccountUpdate {
    /// Account ID.
    account_id: AccountId,

    /// The hash of the account before the transaction was executed.
    ///
    /// Set to `Digest::default()` for new accounts.
    init_state_hash: Digest,

    /// The hash of the account after the transaction was executed.
    final_state_hash: Digest,

    /// Optional account state changes used for on-chain accounts. This data is used to update an
    /// on-chain account's state after a local transaction execution. For private accounts, this
    /// is [AccountUpdateDetails::Private].
    details: AccountUpdateDetails,
}

impl TxAccountUpdate {
    /// Returns a new [TxAccountUpdate] instantiated from the specified components.
    pub const fn new(
        account_id: AccountId,
        init_state_hash: Digest,
        final_state_hash: Digest,
        details: AccountUpdateDetails,
    ) -> Self {
        Self {
            account_id,
            init_state_hash,
            final_state_hash,
            details,
        }
    }

    /// Returns the ID of the updated account.
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Returns the initial account state hash.
    pub fn init_state_hash(&self) -> Digest {
        self.init_state_hash
    }

    /// Returns the hash of the account after the transaction was executed.
    pub fn final_state_hash(&self) -> Digest {
        self.final_state_hash
    }

    /// Returns the account update details.
    pub fn details(&self) -> &AccountUpdateDetails {
        &self.details
    }

    /// Returns `true` if the account update details are for private account.
    pub fn is_private(&self) -> bool {
        self.details.is_private()
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for TxAccountUpdate {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.account_id.write_into(target);
        self.init_state_hash.write_into(target);
        self.final_state_hash.write_into(target);
        self.details.write_into(target);
    }
}

impl Deserializable for TxAccountUpdate {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        Ok(Self {
            account_id: AccountId::read_from(source)?,
            init_state_hash: Digest::read_from(source)?,
            final_state_hash: Digest::read_from(source)?,
            details: AccountUpdateDetails::read_from(source)?,
        })
    }
}
