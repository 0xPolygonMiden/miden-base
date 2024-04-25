use crate::{
    accounts::{delta::AccountUpdateDetails, AccountId},
    block::BlockAccountUpdate,
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest,
};

/// Account update data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxAccountUpdate {
    /// The hash of the account before the transaction was executed.
    ///
    /// Set to `Digest::default()` for new accounts.
    init_state_hash: Digest,

    /// Account update information.
    update: BlockAccountUpdate,
}

impl TxAccountUpdate {
    /// Creates a new [TxAccountUpdate].
    pub const fn new(
        account_id: AccountId,
        init_state_hash: Digest,
        new_state_hash: Digest,
        details: AccountUpdateDetails,
    ) -> Self {
        Self {
            init_state_hash,
            update: BlockAccountUpdate::new(account_id, new_state_hash, details),
        }
    }

    /// Returns the account ID.
    pub fn account_id(&self) -> AccountId {
        self.update.account_id()
    }

    /// Returns the initial account state hash.
    pub fn init_state_hash(&self) -> Digest {
        self.init_state_hash
    }

    /// Returns the hash of the account after the transaction was executed.
    pub fn new_state_hash(&self) -> Digest {
        self.update.new_state_hash()
    }

    /// Returns the account update details.
    pub fn details(&self) -> &AccountUpdateDetails {
        self.update.details()
    }

    /// Returns `true` if the account update details are for private account.
    pub fn is_private(&self) -> bool {
        self.update.is_private()
    }

    /// Returns the account update.
    pub fn update(&self) -> &BlockAccountUpdate {
        &self.update
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for TxAccountUpdate {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.init_state_hash.write_into(target);
        self.update.write_into(target);
    }
}

impl Deserializable for TxAccountUpdate {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        Ok(Self {
            init_state_hash: Digest::read_from(source)?,
            update: BlockAccountUpdate::read_from(source)?,
        })
    }
}
